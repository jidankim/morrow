#!/usr/bin/env bash

qa_usage() {
  cat <<'USAGE'
usage: scripts/run-lifecycle-real-surface-qa.sh --out-dir <path> [--fixture-status PASS|BLOCKED|FAIL|PASS_TIMEOUT|PASS_NONZERO|CALENDAR_EVENT_CREATE_FAIL|CALENDAR_EVENT_CREATE_ACCESS_DENIED|CALENDAR_EVENT_CREATE_PRODUCTION_ACCESS_DENIED]

Runs real Calendar lifecycle QA and real Reminders EventKit QA, then writes:
  calendar-status.txt
  reminders-status.txt
  cleanup-receipt.txt
  summary.txt

Exit code is 0 only when both surfaces are PASS or BLOCKED. Any FAIL exits nonzero.
PASS_TIMEOUT and PASS_NONZERO are parser regression fixtures for PASS-looking logs with untrusted exits.
CALENDAR_EVENT_CREATE_* values are parser regression fixtures for post-save Calendar failures.
USAGE
}

qa_fail_usage() {
  printf 'FAIL: %s\n' "$1" >&2
  qa_usage >&2
  exit 64
}

source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/lifecycle-real-surface-qa-output-guard.sh"

qa_prepare_out_dir() {
  local root="$1"
  local output="$2"
  local output_abs
  local phase_out_dir_status

  output_abs="$(qa_canonicalize_out_dir "$root" "$output")" || return 1

  case "$output_abs" in
    ""|"/"|"$root") qa_fail_usage "refusing unsafe --out-dir: $output" ;;
  esac

  qa_is_allowed_phase_out_dir "$root" "$output_abs" "$output"
  phase_out_dir_status=$?
  if (( phase_out_dir_status == 1 )); then
    return 1
  fi

  if (( phase_out_dir_status != 0 )); then
    qa_is_allowed_tmp_out_dir "$output_abs" ||
      qa_fail_usage "refusing to clean output outside dedicated phase real-surface evidence or wrapper tmp dir: $output"
    qa_require_empty_or_marked_tmp_dir "$output_abs"
  fi

  mkdir -p "$output_abs" || return 1
  qa_clean_out_dir "$output_abs"
  printf 'lifecycle real-surface qa out-dir\n' >"$output_abs/.morrow-lifecycle-real-surface-qa-out-dir"
}

qa_truncate_receipts() {
  local file

  for file in "$@"; do
    : > "$file"
  done
}

qa_run_real_surface() {
  local root="$1"
  local command="$2"
  local raw_file="$3"
  local pid
  local status
  local deadline

  (
    cd "$root" || exit 1
    "$command"
  ) >"$raw_file" 2>&1 &

  pid=$!
  deadline=$((SECONDS + 240))
  while kill -0 "$pid" 2>/dev/null; do
    if (( SECONDS >= deadline )); then
      kill "$pid" 2>/dev/null || true
      sleep 1
      kill -9 "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null
      printf '\nWRAPPER_TIMEOUT: command exceeded 240 seconds and was stopped before status trust.\n' >>"$raw_file"
      return 124
    fi
    sleep 1
  done

  wait "$pid"
  status=$?
  printf '\ncommand_exit=%s\n' "$status" >>"$raw_file"
  return "$status"
}

qa_write_fixture_raw() {
  local surface="$1"
  local status="$2"
  local raw_file="$3"
  local output="$4"

  case "$status" in
    PASS) qa_write_pass_fixture "$surface" "$raw_file" ;;
    BLOCKED)
      cat >"$raw_file" <<EOF
scenario=fixture-$surface-eventkit
status=blocked
reason=fixture_permission_blocked_before_mutation
required_action=Grant $surface access for the command-line QA process, then rerun scripts/run-lifecycle-real-surface-qa.sh --out-dir $output
BLOCKED: fixture blocked before mutation
command_exit=77
EOF
      ;;
    FAIL)
      cat >"$raw_file" <<EOF
scenario=fixture-$surface-eventkit
status=failed
reason=fixture_contract_failure
cleanup_test_items_remaining=1
FAIL: fixture parser failure
command_exit=1
EOF
      ;;
    PASS_TIMEOUT)
      qa_write_pass_fixture "$surface" "$raw_file"
      cat >>"$raw_file" <<'EOF'
WRAPPER_TIMEOUT: command exceeded 240 seconds and was stopped before status trust.
underlying_exit=124
EOF
      ;;
    PASS_NONZERO)
      qa_write_pass_fixture "$surface" "$raw_file"
      cat >>"$raw_file" <<'EOF'
underlying_exit=2
EOF
      ;;
    CALENDAR_EVENT_CREATE_FAIL)
      if [[ "$surface" == "calendar" ]]; then
        cat >"$raw_file" <<'EOF'
scenario=fixture-calendar-event-create-failed-after-save
status=failed
save_observation=saveEvent:commit:YES attempted
event create failed: missing EventKit identifier
cleanup_test_events_remaining=1
FAIL calendar_lifecycle_real_qa
command_exit=1
EOF
      else
        qa_write_pass_fixture "$surface" "$raw_file"
      fi
      ;;
    CALENDAR_EVENT_CREATE_ACCESS_DENIED)
      if [[ "$surface" == "calendar" ]]; then
        cat >"$raw_file" <<'EOF'
scenario=fixture-calendar-event-create-access-denied-after-save
status=failed
save_observation=saveEvent:commit:YES attempted
event create failed: access denied by EventKit after save attempt
cleanup_test_events_remaining=1
FAIL calendar_lifecycle_real_qa
command_exit=1
EOF
      else
        qa_write_pass_fixture "$surface" "$raw_file"
      fi
      ;;
    CALENDAR_EVENT_CREATE_PRODUCTION_ACCESS_DENIED)
      if [[ "$surface" == "calendar" ]]; then
        cat >"$raw_file" <<'EOF'
event create failed: access denied by EventKit after save attempt
permission probe: Calendar access was already granted before event mutation
command_exit=1
EOF
      else
        qa_write_pass_fixture "$surface" "$raw_file"
      fi
      ;;
  esac
}

qa_write_pass_fixture() {
  local surface="$1"
  local raw_file="$2"

  if [[ "$surface" == "calendar" ]]; then
    cat >"$raw_file" <<'EOF'
scenario=fixture-calendar-lifecycle-eventkit
status=passed
move_observation=approved_by_move_to_controlled_calendar
delete_observation=rejected_by_delete_from_proposed
copy_observation=approved_by_copy_to_controlled_calendar_source_proposal_removed
past_event_observation=approved_past_event_preserved_after_end_time
cleanup_test_events_removed=4
cleanup_test_events_remaining=0
PASS calendar_lifecycle_real_qa
command_exit=0
EOF
  else
    cat >"$raw_file" <<'EOF'
scenario=fixture-reminders-eventkit
status=passed
date_only_readback=2026-07-17 time=absent
explicit_time_readback=2026-07-18 09:30
completion_observation=rejected_resolved_in_proposed
move_observation=approved_by_move_to_controlled_list
delete_observation=rejected_by_delete_from_proposed
copy_observation=approved_by_copy_to_controlled_list_source_proposal_removed
cleanup_test_items_removed=5
cleanup_test_items_remaining=0
PASS reminders_real_qa
command_exit=0
EOF
  fi
}

qa_extract_status() {
  local file="$1"
  sed -n 's/^status=//p' "$file" | head -n 1
}

qa_write_cleanup_receipt() {
  local calendar_status="$1"
  local reminders_status="$2"

  {
    printf 'run_id=%s\n' "$run_id"
    printf 'invocation=%s\n' "$invocation"
    printf 'calendar_status=%s\n' "$calendar_status"
    grep -E '^proof=' "$calendar_status_file" | sed 's/^/calendar_/'
    grep -E '^required_user_action=' "$calendar_status_file" | sed 's/^/calendar_/'
    printf 'reminders_status=%s\n' "$reminders_status"
    grep -E '^proof=' "$reminders_status_file" | sed 's/^/reminders_/'
    grep -E '^required_user_action=' "$reminders_status_file" | sed 's/^/reminders_/'
    if [[ "$calendar_status" == "PASS" && "$reminders_status" == "PASS" ]]; then
      printf 'overall_cleanup=cleanup proved for Calendar and Reminders real QA artifacts\n'
    elif [[ "$calendar_status" == "FAIL" || "$reminders_status" == "FAIL" ]]; then
      printf 'overall_cleanup=FAIL: cleanup proof is incomplete for at least one runnable surface; inspect raw logs before rerun\n'
    else
      printf 'overall_cleanup=blocked surfaces proved blocked before mutation; PASS surfaces proved cleanup\n'
    fi
  } >"$cleanup_file"
}

qa_write_summary() {
  local id="$1"
  local calendar_status="$2"
  local reminders_status="$3"

  {
    printf 'run_id=%s\n' "$id"
    printf 'invocation=%s\n' "$invocation"
    printf 'calendar_status=%s\n' "$calendar_status"
    printf 'reminders_status=%s\n' "$reminders_status"
    printf 'status_contract=PASS or BLOCKED per surface exits 0; any FAIL exits nonzero.\n'
    printf 'calendar_receipt=%s\n' "$calendar_status_file"
    printf 'reminders_receipt=%s\n' "$reminders_status_file"
    printf 'cleanup_receipt=%s\n' "$cleanup_file"
    if [[ "$calendar_status" == "PASS" && "$reminders_status" == "PASS" ]]; then
      printf 'overall_status=PASS\n'
      printf 'claim=Full real-surface QA passed for Calendar lifecycle and Reminders create/read/delete cleanup scope.\n'
    elif [[ "$calendar_status" == "FAIL" || "$reminders_status" == "FAIL" ]]; then
      printf 'overall_status=FAIL\n'
      printf 'claim=At least one real surface failed; no full real-surface pass is claimed.\n'
    else
      printf 'overall_status=BLOCKED\n'
      printf 'claim=No full real-surface pass is claimed because at least one surface is BLOCKED.\n'
    fi
    printf 'reminders_lifecycle_scope=Reminders wrapper acceptance is create/read/delete real QA only; reschedule/cancel lifecycle coverage is not claimed.\n'
  } >"$summary_file"
}
