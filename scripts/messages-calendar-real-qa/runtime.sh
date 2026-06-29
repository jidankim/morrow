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

cleanup_event() {
  local source="$1"
  local raw_cleanup="$work_dir/cleanup.out"
  local redacted
  redacted="$(redacted_id "$event_id")"
  if "$eventkit_binary" cleanup "$event_id" > "$raw_cleanup" 2>&1; then
    if grep -q '^CLEANUP_DELETED=true$' "$raw_cleanup"; then
      printf 'cleanup_source=%s\ncleanup_deleted=true\ncreated_event_id=%s\n' "$source" "$redacted" >> "$cleanup_log"
      cleanup_done="true"
      event_id=""
      return 0
    fi
    printf 'cleanup_source=%s\ncleanup_deleted=false\ncreated_event_id=%s\n' "$source" "$redacted" >> "$cleanup_log"
    cleanup_done="true"
    event_id=""
    return 0
  fi
  if grep -q '^BLOCKED_CALENDAR_ACCESS$' "$raw_cleanup"; then
    printf 'cleanup_source=%s\ncleanup_blocked=missing_calendar_access\ncreated_event_id=%s\n' "$source" "$redacted" >> "$cleanup_log"
    return 20
  fi
  printf 'cleanup_source=%s\ncleanup_failed=true\ncreated_event_id=%s\n' "$source" "$redacted" >> "$cleanup_log"
  return 1
}
