#!/usr/bin/env bash
set -u

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
source "$repo_root/scripts/lifecycle-real-surface-qa-lib.sh"
source "$repo_root/scripts/lifecycle-real-surface-qa-classify.sh"

out_dir=""
fixture_status=""

while (($# > 0)); do
  case "$1" in
    --out-dir)
      (($# >= 2)) || qa_fail_usage "--out-dir requires a path"
      out_dir="$2"
      shift 2
      ;;
    --fixture-status)
      (($# >= 2)) || qa_fail_usage "--fixture-status requires PASS, BLOCKED, FAIL, PASS_TIMEOUT, PASS_NONZERO, CALENDAR_EVENT_CREATE_FAIL, CALENDAR_EVENT_CREATE_ACCESS_DENIED, or CALENDAR_EVENT_CREATE_PRODUCTION_ACCESS_DENIED"
      fixture_status="$2"
      shift 2
      ;;
    -h|--help)
      qa_usage
      exit 0
      ;;
    *)
      qa_fail_usage "unknown argument: $1"
      ;;
  esac
done

[[ -n "$out_dir" ]] || qa_fail_usage "--out-dir is required"

case "$fixture_status" in
  ""|PASS|BLOCKED|FAIL|PASS_TIMEOUT|PASS_NONZERO|CALENDAR_EVENT_CREATE_FAIL|CALENDAR_EVENT_CREATE_ACCESS_DENIED|CALENDAR_EVENT_CREATE_PRODUCTION_ACCESS_DENIED) ;;
  *) qa_fail_usage "--fixture-status must be PASS, BLOCKED, FAIL, PASS_TIMEOUT, PASS_NONZERO, CALENDAR_EVENT_CREATE_FAIL, CALENDAR_EVENT_CREATE_ACCESS_DENIED, or CALENDAR_EVENT_CREATE_PRODUCTION_ACCESS_DENIED" ;;
esac

qa_prepare_out_dir "$repo_root" "$out_dir" || exit 1

run_id="$(date -u '+%Y%m%dT%H%M%SZ')-$$"
invocation="scripts/run-lifecycle-real-surface-qa.sh --out-dir $out_dir"
if [[ -n "$fixture_status" ]]; then
  invocation="$invocation --fixture-status $fixture_status"
fi

calendar_raw="$out_dir/calendar-raw.txt"
reminders_raw="$out_dir/reminders-raw.txt"
calendar_status_file="$out_dir/calendar-status.txt"
reminders_status_file="$out_dir/reminders-status.txt"
cleanup_file="$out_dir/cleanup-receipt.txt"
summary_file="$out_dir/summary.txt"

calendar_command="crates/morrow-calendar/scripts/calendar_lifecycle_real_qa.sh"
reminders_command="crates/morrow-reminders/scripts/reminders_real_qa.sh"

qa_truncate_receipts \
  "$calendar_raw" \
  "$reminders_raw" \
  "$calendar_status_file" \
  "$reminders_status_file" \
  "$cleanup_file" \
  "$summary_file"

if [[ -n "$fixture_status" ]]; then
  qa_write_fixture_raw "calendar" "$fixture_status" "$calendar_raw" "$out_dir"
  qa_write_fixture_raw "reminders" "$fixture_status" "$reminders_raw" "$out_dir"
  case "$fixture_status" in
    PASS)
      calendar_exit=0
      reminders_exit=0
      ;;
    BLOCKED)
      calendar_exit=77
      reminders_exit=77
      ;;
    FAIL)
      calendar_exit=1
      reminders_exit=1
      ;;
    PASS_TIMEOUT)
      calendar_exit=124
      reminders_exit=124
      ;;
    PASS_NONZERO)
      calendar_exit=2
      reminders_exit=2
      ;;
    CALENDAR_EVENT_CREATE_FAIL|CALENDAR_EVENT_CREATE_ACCESS_DENIED|CALENDAR_EVENT_CREATE_PRODUCTION_ACCESS_DENIED)
      calendar_exit=1
      reminders_exit=0
      ;;
  esac
else
  qa_run_real_surface "$repo_root" "$calendar_command" "$calendar_raw"
  calendar_exit=$?
  qa_run_real_surface "$repo_root" "$reminders_command" "$reminders_raw"
  reminders_exit=$?
fi

qa_classify_calendar "$calendar_raw" "$calendar_status_file" "$calendar_exit"
qa_classify_reminders "$reminders_raw" "$reminders_status_file" "$reminders_exit"

calendar_status="$(qa_extract_status "$calendar_status_file")"
reminders_status="$(qa_extract_status "$reminders_status_file")"

qa_write_cleanup_receipt "$calendar_status" "$reminders_status"
qa_write_summary "$run_id" "$calendar_status" "$reminders_status"

cat "$summary_file"

if [[ "$calendar_status" == "FAIL" || "$reminders_status" == "FAIL" ]]; then
  exit 1
fi

exit 0
