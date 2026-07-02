#!/usr/bin/env bash

qa_has_line() {
  local pattern="$1"
  local file="$2"
  grep -Eq "$pattern" "$file"
}

qa_has_timeout_or_nonzero_exit() {
  local raw_file="$1"
  local command_status="$2"

  [[ "$command_status" != "0" ]] ||
    qa_has_line '^WRAPPER_TIMEOUT:' "$raw_file" ||
    qa_has_line '^(command_exit|underlying_exit)=([1-9][0-9]*|-)' "$raw_file"
}

qa_calendar_required_action() {
  local status="$1"
  local reason="$2"

  case "$status" in
    PASS) printf 'none; real Calendar lifecycle write/read/cleanup proof captured' ;;
    BLOCKED)
      if [[ "$reason" == *"clang"* || "$reason" == *"xcrun"* || "$reason" == *"SDK"* || "$reason" == *"framework"* || "$reason" == *"utility"* ]]; then
        printf 'Install/repair Xcode Command Line Tools and EventKit SDK availability, then rerun %s' "$invocation"
      else
        printf 'Grant Calendar access to this terminal/Codex command-line process, then rerun %s' "$invocation"
      fi
      ;;
    *) printf 'Inspect %s, fix the Calendar lifecycle failure, confirm cleanup, then rerun %s' "$calendar_raw" "$invocation" ;;
  esac
}

qa_calendar_blocker_pattern() {
  printf '%s\n' '^status=blocked$|^BLOCKED:|permission|not authorized|Not authorized|authorization denied|access denied|Calendar access|requires macOS|framework not found|unable to find utility|xcrun|SDK|clang|Command Line Tools|toolchain'
}

qa_classify_calendar() {
  local raw_file="$1"
  local status_file="$2"
  local command_status="$3"
  local status="FAIL"
  local reason="calendar_contract_failed"
  local mutation_proof="cleanup_not_proven"
  local blocker_pattern

  if qa_calendar_passed "$raw_file" && ! qa_has_timeout_or_nonzero_exit "$raw_file" "$command_status"; then
    status="PASS"
    reason="real_calendar_lifecycle_write_read_cleanup_proved"
    mutation_proof="cleanup_completed: cleanup_test_events_remaining=0"
  else
    blocker_pattern="$(qa_calendar_blocker_pattern)"
    if qa_has_timeout_or_nonzero_exit "$raw_file" "$command_status"; then
      reason="calendar_timeout_or_nonzero_underlying_exit"
    fi
  fi

  if [[ "$status" == "FAIL" ]] && qa_calendar_post_save_failure "$raw_file"; then
    reason="calendar_post_save_event_create_failed"
  elif [[ "$status" == "FAIL" ]] && ! qa_calendar_passed "$raw_file" && qa_has_line "$blocker_pattern" "$raw_file"; then
    status="BLOCKED"
    reason="$(grep -E "$blocker_pattern" "$raw_file" | head -n 1)"
    mutation_proof="blocked_before_mutation: Calendar authorization/tool availability is checked before controlled calendars/events are created"
  fi

  {
    printf 'surface=calendar\n'
    printf 'status=%s\n' "$status"
    printf 'status_contract=PASS requires status=passed, PASS calendar_lifecycle_real_qa, lifecycle readbacks, and cleanup_test_events_remaining=0; BLOCKED requires OS/tool/permission blocker and rerun command; FAIL means runnable behavior or cleanup proof violated.\n'
    printf 'reason=%s\n' "$reason"
    printf 'underlying_command=%s\n' "$calendar_command"
    printf 'underlying_exit=%s\n' "$command_status"
    printf 'rerun_command=%s\n' "$invocation"
    printf 'required_user_action=%s\n' "$(qa_calendar_required_action "$status" "$reason")"
    printf 'proof=%s\n' "$mutation_proof"
    printf 'raw_log=%s\n' "$raw_file"
  } >"$status_file"
}

qa_calendar_passed() {
  local raw_file="$1"

  qa_has_line '^status=passed$' "$raw_file" &&
    qa_has_line '^PASS calendar_lifecycle_real_qa$' "$raw_file" &&
    qa_has_line '^move_observation=approved_by_move_to_controlled_calendar$' "$raw_file" &&
    qa_has_line '^delete_observation=rejected_by_delete_from_proposed$' "$raw_file" &&
    qa_has_line '^copy_observation=approved_by_copy_to_controlled_calendar_source_proposal_removed$' "$raw_file" &&
    qa_has_line '^past_event_observation=approved_past_event_preserved_after_end_time$' "$raw_file" &&
    qa_has_line '^cleanup_test_events_removed=[0-9]+$' "$raw_file" &&
    qa_has_line '^cleanup_test_events_remaining=0$' "$raw_file"
}

qa_calendar_post_save_failure() {
  local raw_file="$1"

  qa_has_line '^event create failed:' "$raw_file" ||
    (
      qa_has_line '^status=failed$' "$raw_file" &&
        qa_has_line '^save_observation=saveEvent:commit:YES attempted$' "$raw_file"
    )
}

qa_classify_reminders() {
  local raw_file="$1"
  local status_file="$2"
  local command_status="$3"
  local status="FAIL"
  local reason="reminders_contract_failed"
  local mutation_proof="cleanup_not_proven"
  local required_action

  if qa_reminders_passed "$raw_file" && ! qa_has_timeout_or_nonzero_exit "$raw_file" "$command_status"; then
    status="PASS"
    reason="real_reminders_create_read_delete_cleanup_proved"
    mutation_proof="cleanup_completed: cleanup_test_items_remaining=0"
  elif qa_reminders_passed "$raw_file" && qa_has_timeout_or_nonzero_exit "$raw_file" "$command_status"; then
    reason="reminders_timeout_or_nonzero_underlying_exit"
  elif qa_has_line '^status=blocked$|BLOCKED:|required_action=|reminders_access_|not_determined|permission|access' "$raw_file"; then
    status="BLOCKED"
    reason="$(grep -E '^reason=|BLOCKED:|reminders_access_|not_determined|permission|access' "$raw_file" | head -n 1 | sed 's/^reason=//')"
    mutation_proof="blocked_before_mutation: Reminders authorization is checked before controlled lists/reminders are created"
  elif qa_has_line 'framework not found|unable to find utility|xcrun|SDK|clang' "$raw_file"; then
    status="BLOCKED"
    reason="$(grep -E 'framework not found|unable to find utility|xcrun|SDK|clang' "$raw_file" | head -n 1)"
    mutation_proof="blocked_before_mutation: toolchain failure occurred before QA binary execution"
  fi

  required_action="$(grep -E '^required_action=' "$raw_file" | head -n 1 | sed 's/^required_action=//')"
  if [[ -z "$required_action" ]]; then
    required_action="$(qa_default_reminders_action "$status")"
  fi

  {
    printf 'surface=reminders\n'
    printf 'status=%s\n' "$status"
    printf 'status_contract=PASS requires status=passed, PASS reminders_real_qa, create/read/delete observations, and cleanup_test_items_remaining=0; BLOCKED requires OS/tool/permission blocker and rerun command; FAIL means runnable behavior or cleanup proof violated.\n'
    printf 'coverage_scope=create/read/delete real QA only; lifecycle reschedule/cancel coverage is not claimed by this wrapper.\n'
    printf 'reason=%s\n' "$reason"
    printf 'underlying_command=%s\n' "$reminders_command"
    printf 'underlying_exit=%s\n' "$command_status"
    printf 'rerun_command=%s\n' "$invocation"
    printf 'required_user_action=%s\n' "$required_action"
    printf 'proof=%s\n' "$mutation_proof"
    printf 'raw_log=%s\n' "$raw_file"
  } >"$status_file"
}

qa_reminders_passed() {
  local raw_file="$1"

  qa_has_line '^status=passed$' "$raw_file" &&
    qa_has_line '^PASS reminders_real_qa$' "$raw_file" &&
    qa_has_line '^date_only_readback=2026-07-17 time=absent$' "$raw_file" &&
    qa_has_line '^explicit_time_readback=2026-07-18 09:30$' "$raw_file" &&
    qa_has_line '^completion_observation=rejected_resolved_in_proposed$' "$raw_file" &&
    qa_has_line '^move_observation=approved_by_move_to_controlled_list$' "$raw_file" &&
    qa_has_line '^delete_observation=rejected_by_delete_from_proposed$' "$raw_file" &&
    qa_has_line '^copy_observation=approved_by_copy_to_controlled_list_source_proposal_removed$' "$raw_file" &&
    qa_has_line '^cleanup_test_items_removed=[0-9]+$' "$raw_file" &&
    qa_has_line '^cleanup_test_items_remaining=0$' "$raw_file"
}

qa_default_reminders_action() {
  local status="$1"

  case "$status" in
    PASS) printf 'none; real Reminders create/read/delete/cleanup proof captured' ;;
    BLOCKED) printf 'Grant Reminders access to this terminal/Codex command-line process or repair the macOS EventKit toolchain, then rerun %s' "$invocation" ;;
    *) printf 'Inspect %s, fix the Reminders real QA failure, confirm cleanup, then rerun %s' "$reminders_raw" "$invocation" ;;
  esac
}
