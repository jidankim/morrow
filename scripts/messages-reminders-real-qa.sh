#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
helper_dir="$repo_root/scripts/messages-reminders-real-qa"
evidence_dir="${MORROW_REAL_QA_EVIDENCE_DIR:-.omo/evidence/real-reminders-proposal-replay/task-7-real-reminders-qa}"
evidence_dir="$repo_root/$evidence_dir"
evidence_root="$repo_root/.omo/evidence/real-reminders-proposal-replay"
storage_db="$evidence_dir/morrow-real-reminders-qa.sqlite"
run_log="$evidence_dir/real-reminders-qa-run.log"
readback_json="$evidence_dir/real-reminders-qa-readback.json"
cleanup_log="$evidence_dir/real-reminders-qa-cleanup.log"
privacy_log="$evidence_dir/privacy-scan.log"
helper_log="$evidence_dir/native-scan-helper.log"
blocked_cases_log="$evidence_root/task-7-real-reminders-qa-blocked-cases.log"
forbidden_tokens="$evidence_root/forbidden-tokens.txt"
work_dir="$(mktemp -d)"
example_name="morrow_real_reminders_qa_$$"
example_path="$repo_root/src-tauri/examples/${example_name}.rs"
eventkit_binary="$work_dir/morrow-real-reminders-qa-eventkit"
reminder_id=""
candidate_id=""
redacted=""
cleanup_done="false"

cleanup_temp() {
  if [[ -n "$reminder_id" && "$cleanup_done" != "true" && -x "$eventkit_binary" ]]; then
    cleanup_reminder "trap" || true
  fi
  rm -f "$example_path"
  rm -rf "$work_dir"
}
trap cleanup_temp EXIT

source "$helper_dir/evidence.sh"
source "$helper_dir/rust-helper.sh"
source "$helper_dir/eventkit-helper.sh"
source "$helper_dir/runtime.sh"

main() {
  write_base_evidence
  validate_inputs
  init_storage_db
  record_run "scenario" "real_messages_to_reminders_qa"
  record_run "messages_surface" "native_messages_sqlite_selected_chat"
  record_run "provider_surface" "codex_cli_session"
  record_run "reminders_surface" "eventkit_reminder_proposal_bridge"
  record_run "message_body_dumped" "false"
  record_run "raw_handles_dumped" "false"
  record_run "raw_reminder_ids_dumped" "false"
  record_run "api_key_dumped" "false"
  record_run "provider_payload_dumped" "false"
  record_run "codex_stdout_dumped" "false"
  record_run "codex_stderr_dumped" "false"

  write_rust_helper
  if ! write_eventkit_helper; then
    fail_sanitized "EventKit Reminders helper compile failed"
  fi

  export MORROW_REAL_QA_STORE_PATH="$storage_db"
  export SDKROOT="${SDKROOT:-/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk}"
  export RUSTFLAGS="${RUSTFLAGS:--C linker=/Library/Developer/CommandLineTools/usr/bin/cc}"
  local helper_output="$work_dir/rust-helper.out"
  local timeout_seconds="${MORROW_REAL_QA_TIMEOUT_SECONDS:-90}"
  if ! (cd "$repo_root/src-tauri" && run_with_timeout "$timeout_seconds" "$helper_output" cargo run --quiet --example "$example_name"); then
    cp "$helper_output" "$helper_log"
    handle_helper_status "$helper_output"
    fail_sanitized "native Reminders scan helper failed"
  fi
  cp "$helper_output" "$helper_log"

  handle_helper_status "$helper_output"
  record_helper_counts "$helper_output"
  record_created_reminder
  validate_readback
  finalize_success
}

handle_helper_status() {
  local helper_output="$1"
  if grep -q '^STATUS=BLOCKED_FULL_DISK_ACCESS$' "$helper_output"; then
    emit_blocked "missing Full Disk Access"
  fi
  if grep -q '^STATUS=BLOCKED_TEST_CHAT_NOT_FOUND$' "$helper_output"; then
    emit_blocked "test chat not found"
  fi
  if grep -q '^STATUS=BLOCKED_REMINDERS_ACCESS$' "$helper_output"; then
    emit_blocked "missing Reminders access"
  fi
  if grep -q '^STATUS=BLOCKED_CODEX_CLI_MISSING$' "$helper_output"; then
    emit_blocked "missing Codex CLI"
  fi
  if grep -q '^STATUS=BLOCKED_CODEX_LOGIN_REQUIRED$' "$helper_output"; then
    emit_blocked "missing Codex login"
  fi
  if grep -q '^STATUS=BLOCKED_CODEX_AUTH_TIMEOUT$' "$helper_output"; then
    emit_blocked "Codex auth check timed out"
  fi
  if grep -q '^STATUS=BLOCKED_CODEX_AUTH_UNAVAILABLE$' "$helper_output"; then
    emit_blocked "Codex auth check failed"
  fi
  if grep -q '^STATUS=NO_REMINDER_CREATED$' "$helper_output"; then
    fail_sanitized "no Reminders proposal was created"
  fi
  if ! grep -q '^STATUS=SCAN_OK$' "$helper_output"; then
    fail_sanitized "native scan did not report Reminders success"
  fi
}

record_helper_counts() {
  local helper_output="$1"
  while IFS= read -r line; do
    case "$line" in
      CREATED_EXTERNAL=*|FAILED_EXTERNAL=*|CREATED_CANDIDATES=*|QUIET_LOGS=*|VISIBLE_TASK_REMINDERS=*|REMINDER_RECEIPTS=*|REMINDER_MAPPINGS=*)
        printf '%s\n' "$line" | tr '[:upper:]' '[:lower:]' >> "$run_log"
        ;;
    esac
  done < "$helper_output"
}

record_created_reminder() {
  reminder_id="$(sqlite3 -batch -noheader "$storage_db" "SELECT c.external_object_id FROM candidates c JOIN external_object_mappings m ON m.candidate_id = c.id AND m.source = 'reminders' WHERE c.kind = 'task_reminder' AND c.state = 'visible' AND c.external_object_id IS NOT NULL AND c.external_source_id IS NOT NULL ORDER BY c.updated_at DESC, c.id DESC LIMIT 1;" | head -n 1)"
  candidate_id="$(sqlite3 -batch -noheader "$storage_db" "SELECT c.id FROM candidates c JOIN external_object_mappings m ON m.candidate_id = c.id AND m.source = 'reminders' WHERE c.kind = 'task_reminder' AND c.state = 'visible' AND c.external_object_id IS NOT NULL AND c.external_source_id IS NOT NULL ORDER BY c.updated_at DESC, c.id DESC LIMIT 1;" | head -n 1)"
  if [[ -z "$reminder_id" || -z "$candidate_id" ]]; then
    fail_sanitized "created Reminders identifier missing from storage"
  fi
  redacted="$(redacted_id "$reminder_id")"
  record_run "created_reminder_id" "$redacted"
}

validate_readback() {
  local readback_tmp="$work_dir/readback.json"
  if ! "$eventkit_binary" readback "$reminder_id" "$candidate_id" "$MORROW_REAL_QA_EXPECTED_TITLE_CONTAINS" "$MORROW_REAL_QA_FUTURE_ISO_LOCAL" > "$readback_tmp" 2>&1; then
    if grep -q '^BLOCKED_REMINDERS_ACCESS$' "$readback_tmp"; then
      cleanup_reminder "readback_blocked" || true
      emit_blocked "missing Reminders access"
    fi
    cp "$readback_tmp" "$readback_json"
    cleanup_reminder "readback_failed" || true
    fail_sanitized "EventKit Reminders readback validation failed"
  fi
  cp "$readback_tmp" "$readback_json"
  record_run "readback" "ok"
}

finalize_success() {
  if ! cleanup_reminder "pass"; then
    fail_sanitized "EventKit Reminders cleanup failed"
  fi
  record_run "cleanup" "ok"

  if ! run_privacy_scan; then
    fail_sanitized "privacy scan failed"
  fi
  record_run "privacy_scan" "ok"
  cp "$run_log" "$evidence_root/task-7-real-reminders-qa-pass.log"
  cp "$cleanup_log" "$evidence_root/task-7-real-reminders-qa-cleanup.log"
  cp "$privacy_log" "$evidence_root/task-7-privacy-scan.log"
  printf 'PASS created_reminder_id=%s readback=ok cleanup=ok\n' "$redacted"
}

main "$@"
