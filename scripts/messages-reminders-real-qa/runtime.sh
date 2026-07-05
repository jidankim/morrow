run_with_timeout() {
  local seconds="$1"
  local output_file="$2"
  shift 2
  "$@" > "$output_file" 2>&1 &
  local command_pid=$!
  (
    sleep "$seconds"
    kill -TERM "$command_pid" 2>/dev/null || true
    sleep 2
    kill -KILL "$command_pid" 2>/dev/null || true
  ) &
  local watchdog_pid=$!
  local status=0
  wait "$command_pid" || status=$?
  kill "$watchdog_pid" 2>/dev/null || true
  wait "$watchdog_pid" 2>/dev/null || true
  if [[ "$status" -eq 143 || "$status" -eq 137 ]]; then
    return 124
  fi
  return "$status"
}

cleanup_reminder() {
  local source="$1"
  local raw_cleanup="$work_dir/cleanup.out"
  local redacted_cleanup
  redacted_cleanup="$(redacted_id "$reminder_id")"
  if "$eventkit_binary" cleanup "$reminder_id" "$candidate_id" > "$raw_cleanup" 2>&1; then
    if grep -q '^CLEANUP_DELETED=true$' "$raw_cleanup"; then
      printf 'cleanup_source=%s\ncleanup_deleted=true\ncreated_reminder_id=%s\n' "$source" "$redacted_cleanup" >> "$cleanup_log"
      cleanup_done="true"
      reminder_id=""
      candidate_id=""
      return 0
    fi
    printf 'cleanup_source=%s\ncleanup_deleted=false\ncreated_reminder_id=%s\n' "$source" "$redacted_cleanup" >> "$cleanup_log"
    cleanup_done="true"
    reminder_id=""
    candidate_id=""
    return 0
  fi
  if grep -q '^BLOCKED_REMINDERS_ACCESS$' "$raw_cleanup"; then
    printf 'cleanup_source=%s\ncleanup_blocked=missing_reminders_access\ncreated_reminder_id=%s\n' "$source" "$redacted_cleanup" >> "$cleanup_log"
    return 20
  fi
  printf 'cleanup_source=%s\ncleanup_failed=true\ncreated_reminder_id=%s\n' "$source" "$redacted_cleanup" >> "$cleanup_log"
  return 1
}
