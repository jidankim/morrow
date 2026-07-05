redacted_id() {
  local value="$1"
  local digest
  digest="$(printf '%s' "$value" | shasum -a 256 | awk '{print substr($1,1,12)}')"
  printf 'redacted-%s' "$digest"
}

write_base_evidence() {
  mkdir -p "$evidence_dir" "$evidence_root"
  : > "$run_log"
  : > "$cleanup_log"
  : > "$readback_json"
  rm -f "$privacy_log"
  if [[ ! -f "$forbidden_tokens" ]]; then
    cat > "$forbidden_tokens" <<'TOKENS'
sk-test-secret-should-not-appear
raw-provider-request-payload
raw-provider-response-body
person@example.com
+15555550123
fake-codex-access-token
TOKENS
  fi
  grep -Fxq 'fake-codex-access-token' "$forbidden_tokens" || printf '%s\n' 'fake-codex-access-token' >> "$forbidden_tokens"
  grep -Fxq 'codex_access_token=fake-codex-access-token' "$forbidden_tokens" || printf '%s\n' 'codex_access_token=fake-codex-access-token' >> "$forbidden_tokens"
}

init_storage_db() {
  rm -f "$storage_db"
  sqlite3 "$storage_db" < "$repo_root/crates/morrow-storage/migrations/0001_init.sql"
}

record_run() {
  local key="$1"
  local value="$2"
  printf '%s=%s\n' "$key" "$value" >> "$run_log"
}

run_privacy_scan() {
  local tmp_log="$work_dir/privacy-scan.log"
  rm -f "$privacy_log"
  scripts/privacy-inspect.sh "$storage_db" "$evidence_dir" "$forbidden_tokens" > "$tmp_log" 2>&1
  mv "$tmp_log" "$privacy_log"
}

emit_blocked() {
  local reason="$1"
  write_base_evidence
  if [[ ! -f "$storage_db" ]]; then
    init_storage_db
  fi
  record_run "scenario" "real_messages_to_reminders_qa"
  record_run "provider_surface" "codex_cli_session"
  record_run "outcome" "blocked"
  record_run "blocked_reason" "$reason"
  record_run "message_body_dumped" "false"
  record_run "raw_handles_dumped" "false"
  record_run "raw_reminder_ids_dumped" "false"
  record_run "api_key_dumped" "false"
  record_run "provider_payload_dumped" "false"
  record_run "codex_stdout_dumped" "false"
  record_run "codex_stderr_dumped" "false"
  if run_privacy_scan; then
    :
  else
    printf 'privacy_scan_status=failed\n' >> "$run_log"
  fi
  printf '%s\n' "BLOCKED: $reason" | tee -a "$blocked_cases_log" >/dev/null
  printf 'BLOCKED: %s\n' "$reason"
  exit 0
}

fail_sanitized() {
  local reason="$1"
  record_run "outcome" "fail"
  record_run "failure_reason" "$reason"
  printf 'FAIL messages_reminders_real_qa: %s\n' "$reason" >&2
  exit 1
}

validate_inputs() {
  if [[ -z "${MORROW_REAL_QA_CHAT_PUBLIC_ID:-}" ||
        -z "${MORROW_REAL_QA_EXPECTED_TITLE_CONTAINS:-}" ||
        -z "${MORROW_REAL_QA_FUTURE_ISO_LOCAL:-}" ]]; then
    emit_blocked "missing required real QA env"
  fi
  if [[ ! "$MORROW_REAL_QA_FUTURE_ISO_LOCAL" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}$ ]]; then
    write_base_evidence
    init_storage_db
    fail_sanitized "invalid future timestamp"
  fi
}
