#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
helper_dir="$repo_root/scripts/messages-calendar-real-qa"
evidence_dir="${MORROW_REAL_QA_EVIDENCE_DIR:-.omo/evidence/codex-oauth-provider-auth/task-7}"
evidence_dir="$repo_root/$evidence_dir"
evidence_root="$repo_root/.omo/evidence/codex-oauth-provider-auth"
storage_db="$evidence_dir/morrow-real-qa.sqlite"
run_log="$evidence_dir/real-qa-run.log"
readback_json="$evidence_dir/real-qa-readback.json"
cleanup_log="$evidence_dir/real-qa-cleanup.log"
privacy_log="$evidence_dir/privacy-scan.log"
helper_log="$evidence_dir/native-scan-helper.log"
blocked_cases_log="$evidence_root/task-7-real-qa-blocked-cases.log"
forbidden_tokens="$evidence_root/forbidden-tokens.txt"
work_dir="$(mktemp -d)"
example_name="morrow_real_qa_$$"
example_path="$repo_root/src-tauri/examples/${example_name}.rs"
eventkit_binary="$work_dir/morrow-real-qa-eventkit"
event_id=""
cleanup_done="false"

cleanup_temp() {
  if [[ -n "$event_id" && "$cleanup_done" != "true" && -x "$eventkit_binary" ]]; then
    cleanup_event "trap" || true
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
  record_run "scenario" "real_messages_to_calendar_qa"
  record_run "messages_surface" "native_messages_sqlite_selected_chat"
  record_run "provider_surface" "codex_cli_session"
  record_run "calendar_surface" "eventkit_proposal_bridge"
  record_run "message_body_dumped" "false"
  record_run "raw_handles_dumped" "false"
  record_run "raw_event_ids_dumped" "false"
  record_run "api_key_dumped" "false"
  record_run "provider_payload_dumped" "false"
  record_run "codex_stdout_dumped" "false"
  record_run "codex_stderr_dumped" "false"

  write_rust_helper
  if ! write_eventkit_helper; then
    fail_sanitized "eventkit helper compile failed"
  fi

  export MORROW_REAL_QA_STORE_PATH="$storage_db"
  export SDKROOT="${SDKROOT:-/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk}"
  export RUSTFLAGS="${RUSTFLAGS:--C linker=/Library/Developer/CommandLineTools/usr/bin/cc}"
  local helper_output="$work_dir/rust-helper.out"
  local timeout_seconds="${MORROW_REAL_QA_TIMEOUT_SECONDS:-90}"
  if ! (cd "$repo_root/src-tauri" && run_with_timeout "$timeout_seconds" "$helper_output" cargo run --quiet --example "$example_name"); then
    cp "$helper_output" "$helper_log"
    if grep -q '^STATUS=BLOCKED_FULL_DISK_ACCESS$' "$helper_output"; then
      emit_blocked "missing Full Disk Access"
    fi
    fail_sanitized "native scan helper failed"
  fi
  cp "$helper_output" "$helper_log"

  handle_helper_status "$helper_output"
  record_helper_counts "$helper_output"
  record_created_event
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
  if grep -q '^STATUS=BLOCKED_CALENDAR_ACCESS$' "$helper_output"; then
    local calendar_failure_reason
    calendar_failure_reason="$(latest_calendar_failure_reason || true)"
    if [[ -n "$calendar_failure_reason" ]]; then
      emit_blocked "Calendar proposal failed: $calendar_failure_reason"
    fi
    emit_blocked "missing Calendar access"
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
  if grep -q '^STATUS=NO_EVENT_CREATED$' "$helper_output"; then
    fail_sanitized "no EventKit proposal was created"
  fi
  if ! grep -q '^STATUS=SCAN_OK$' "$helper_output"; then
    fail_sanitized "native scan did not report success"
  fi
}

latest_calendar_failure_reason() {
  local reason
  reason="$(sqlite3 -batch -noheader "$storage_db" "SELECT current_reason FROM candidates WHERE state = 'failed' AND current_reason LIKE 'external_proposal_creation_failed%' ORDER BY updated_at DESC, id DESC LIMIT 1;" | head -n 1)"
  reason="${reason#external_proposal_creation_failed: }"
  if [[ "$reason" == "external_proposal_creation_failed" ]]; then
    reason=""
  fi
  printf '%s\n' "$reason"
}

record_helper_counts() {
  local helper_output="$1"
  while IFS= read -r line; do
    case "$line" in
      CREATED_EXTERNAL=*|FAILED_EXTERNAL=*|CREATED_CANDIDATES=*|QUIET_LOGS=*)
        printf '%s\n' "$line" | tr '[:upper:]' '[:lower:]' >> "$run_log"
        ;;
    esac
  done < "$helper_output"
}

record_created_event() {
  event_id="$(sqlite3 -batch -noheader "$storage_db" "SELECT external_object_id FROM candidates WHERE external_object_id IS NOT NULL ORDER BY updated_at DESC, id DESC LIMIT 1;" | head -n 1)"
  if [[ -z "$event_id" ]]; then
    fail_sanitized "created EventKit identifier missing from storage"
  fi
  redacted="$(redacted_id "$event_id")"
  record_run "created_event_id" "$redacted"
}

validate_readback() {
  local readback_tmp="$work_dir/readback.json"
  if ! "$eventkit_binary" readback "$event_id" "$MORROW_REAL_QA_EXPECTED_TITLE_CONTAINS" "$MORROW_REAL_QA_FUTURE_ISO_LOCAL" > "$readback_tmp" 2>&1; then
    if grep -q '^BLOCKED_CALENDAR_ACCESS$' "$readback_tmp"; then
      cleanup_event "readback_blocked" || true
      emit_blocked "missing Calendar access"
    fi
    cp "$readback_tmp" "$readback_json"
    cleanup_event "readback_failed" || true
    fail_sanitized "EventKit readback validation failed"
  fi
  cp "$readback_tmp" "$readback_json"
  record_run "readback" "ok"
}

finalize_success() {
  if ! cleanup_event "pass"; then
    fail_sanitized "EventKit cleanup failed"
  fi
  record_run "cleanup" "ok"

  if ! run_privacy_scan; then
    fail_sanitized "privacy scan failed"
  fi
  record_run "privacy_scan" "ok"
  cp "$run_log" "$evidence_root/task-7-real-qa-pass.log"
  cp "$cleanup_log" "$evidence_root/task-7-real-qa-cleanup.log"
  cp "$privacy_log" "$evidence_root/task-7-privacy-scan.log"
  printf 'PASS created_event_id=%s readback=ok cleanup=ok\n' "$redacted"
}

main "$@"
