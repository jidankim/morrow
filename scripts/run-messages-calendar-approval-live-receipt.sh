#!/usr/bin/env bash
set -u

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
phase_root=".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval"
preflight_report="$repo_root/$phase_root/preflight/preflight-report.json"
out_dir=""
fixture_case=""
surface_fixture_status="${MORROW_APPROVAL_LIVE_RECEIPT_LIFECYCLE_FIXTURE_STATUS:-}"

usage() {
  cat <<'USAGE'
usage: scripts/run-messages-calendar-approval-live-receipt.sh --out-dir <path> [--fixture-case PASS|BLOCKED|FAIL|MISSING_CLEANUP|PROVIDER_JSON|MISSING_PRIVACY_SCAN]

Records a bounded live Messages-to-Calendar approval trajectory receipt.
Live PASS is allowed only with real controlled proof; missing prerequisites are BLOCKED.
USAGE
}

fail_usage() {
  printf 'FAIL: %s\n' "$1" >&2
  usage >&2
  exit 64
}

while (($# > 0)); do
  case "$1" in
    --out-dir)
      (($# >= 2)) || fail_usage "--out-dir requires a path"
      out_dir="$2"
      shift 2
      ;;
    --fixture-case)
      (($# >= 2)) || fail_usage "--fixture-case requires a value"
      fixture_case="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail_usage "unknown argument: $1"
      ;;
  esac
done

[[ -n "$out_dir" ]] || fail_usage "--out-dir is required"
case "$fixture_case" in
  ""|PASS|BLOCKED|FAIL|MISSING_CLEANUP|PROVIDER_JSON|MISSING_PRIVACY_SCAN) ;;
  *) fail_usage "--fixture-case must be PASS, BLOCKED, FAIL, MISSING_CLEANUP, PROVIDER_JSON, or MISSING_PRIVACY_SCAN" ;;
esac
case "$surface_fixture_status" in
  ""|PASS|BLOCKED|FAIL|PASS_TIMEOUT|PASS_NONZERO|CALENDAR_EVENT_CREATE_FAIL|CALENDAR_EVENT_CREATE_ACCESS_DENIED|CALENDAR_EVENT_CREATE_PRODUCTION_ACCESS_DENIED) ;;
  *) fail_usage "MORROW_APPROVAL_LIVE_RECEIPT_LIFECYCLE_FIXTURE_STATUS has an unsupported value" ;;
esac

summary_file="$repo_root/$out_dir/summary.txt"
provider_file="$repo_root/$out_dir/provider-readiness.txt"
messages_file="$repo_root/$out_dir/messages-access.txt"
calendar_file="$repo_root/$out_dir/calendar-reminders-access.txt"
cleanup_file="$repo_root/$out_dir/cleanup-receipt.txt"
privacy_file="$repo_root/$out_dir/privacy-inspect.txt"
work_dir=""
lifecycle_work_dir=""
source "$repo_root/scripts/messages-calendar-approval-live-receipt-lib.sh"

cleanup_tmp() {
  [[ -n "$work_dir" ]] && rm -rf -- "$work_dir"
  [[ -n "$lifecycle_work_dir" ]] && rm -rf -- "$lifecycle_work_dir"
}
trap cleanup_tmp EXIT

prepare_out_dir() {
  case "$out_dir" in
    ""|"/"|*"/../"*|*".."|".") fail_usage "refusing unsafe --out-dir: $out_dir" ;;
  esac
  local receipt_path="${out_dir#"$phase_root"/}"
  if [[ "$receipt_path" == "$out_dir" || ! "$receipt_path" =~ ^live-receipt(-[a-z0-9-]+)?(/[A-Za-z0-9._-]+)*$ ]]; then
    fail_usage "--out-dir must be under $phase_root/live-receipt or a live-receipt-* sibling"
  fi
  mkdir -p -- "$repo_root/$out_dir"
  : > "$summary_file"
  : > "$provider_file"
  : > "$messages_file"
  : > "$calendar_file"
  rm -f -- "$cleanup_file" "$privacy_file" "$repo_root/$out_dir/provider-raw.txt" "$repo_root/$out_dir/provider-raw.json"
}

run_bounded() {
  local seconds="$1"
  local output_file="$2"
  shift 2
  "$@" > "$output_file" 2>&1 &
  local pid=$!
  local deadline=$((SECONDS + seconds))
  while kill -0 "$pid" 2>/dev/null; do
    if (( SECONDS >= deadline )); then
      kill "$pid" 2>/dev/null || true
      sleep 1
      kill -9 "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null
      return 124
    fi
    sleep 1
  done
  wait "$pid"
}

probe_provider() {
  local raw="$work_dir/provider.out"
  if ! command -v codex >/dev/null 2>&1; then
    provider_status=BLOCKED
    printf 'provider_readiness_status=BLOCKED\nreason=missing_codex_cli\n' > "$provider_file"
    return 10
  fi
  run_bounded 10 "$raw" codex login status
  local status=$?
  local lower
  lower="$(tr '[:upper:]' '[:lower:]' < "$raw")"
  if (( status == 124 )); then
    provider_status=BLOCKED
    printf 'provider_readiness_status=BLOCKED\nreason=codex_login_status_timeout\n' > "$provider_file"
    return 10
  fi
  if (( status == 0 )) && [[ "$lower" == *"logged in"* && "$lower" == *"chatgpt"* ]]; then
    provider_status=PASS
    printf 'provider_readiness_status=PASS\nprovider_surface=codex_cli_session\n' > "$provider_file"
    return 0
  fi
  provider_status=BLOCKED
  printf 'provider_readiness_status=BLOCKED\nreason=codex_cli_not_logged_in_with_chatgpt\n' > "$provider_file"
  return 10
}

probe_messages() {
  local raw="$work_dir/messages.out"
  run_bounded 60 "$raw" "$repo_root/scripts/messages-discovery-real-qa.sh"
  local status=$?
  if grep -q '^PASS messages_discovery_real_qa$' "$raw"; then
    messages_status=PASS
    printf 'messages_access_status=PASS\nmessage_body_dumped=false\nraw_handles_dumped=false\n' > "$messages_file"
    return 0
  fi
  if grep -q '^BLOCKED:' "$raw"; then
    messages_status=BLOCKED
    printf 'messages_access_status=BLOCKED\nreason=full_disk_access_required\nmessage_body_dumped=false\nraw_handles_dumped=false\n' > "$messages_file"
    return 10
  fi
  messages_status=FAIL
  printf 'messages_access_status=FAIL\nreason=messages_discovery_failed\ncommand_exit=%s\n' "$status" > "$messages_file"
  return 1
}

probe_calendar_reminders() {
  local receipt_abs="$repo_root/$out_dir"
  local -a lifecycle_command
  lifecycle_work_dir="$(mktemp -d "$receipt_abs/morrow-lifecycle-real-surface-qa-XXXXXX")" || return 1
  lifecycle_command=("$repo_root/scripts/run-lifecycle-real-surface-qa.sh" --out-dir "$lifecycle_work_dir")
  if [[ -n "$surface_fixture_status" ]]; then
    lifecycle_command+=("--fixture-status" "$surface_fixture_status")
  fi
  run_bounded 260 "$work_dir/lifecycle.out" env TMPDIR="$receipt_abs" "${lifecycle_command[@]}"
  local status=$?
  if [[ -s "$lifecycle_work_dir/summary.txt" ]]; then
    calendar_status="$(grep '^calendar_status=' "$lifecycle_work_dir/summary.txt" | cut -d= -f2-)"
    reminders_status="$(grep '^reminders_status=' "$lifecycle_work_dir/summary.txt" | cut -d= -f2-)"
  else
    calendar_status=FAIL
    reminders_status=FAIL
  fi
  {
    printf 'calendar_access_status=%s\n' "$calendar_status"
    printf 'reminders_access_status=%s\n' "$reminders_status"
    if [[ -n "$surface_fixture_status" ]]; then
      printf 'calendar_probe_source=test_fixture\n'
    fi
    printf 'command_exit=%s\n' "$status"
  } > "$calendar_file"
  [[ "$calendar_status" != "FAIL" && "$reminders_status" != "FAIL" ]]
}

run_live() {
  work_dir="$(mktemp -d)"
  provider_status=BLOCKED
  messages_status=BLOCKED
  selected_chat_status=BLOCKED
  calendar_status=BLOCKED
  reminders_status=BLOCKED
  proposal_status=BLOCKED
  readback_status=BLOCKED
  qa_action_status=BLOCKED
  reconcile_status=BLOCKED
  idempotency_status=BLOCKED
  controlled_proposal_created=false
  if [[ -s "$preflight_report" ]] && ! grep -q '"live_gate_provider_auth": "Codex CLI session"' "$preflight_report"; then
    finalize_fail "preflight_provider_auth_source_of_truth_is_not_codex_cli_session"
  fi
  probe_provider || finalize_blocked "provider_readiness:$(grep '^reason=' "$provider_file" | cut -d= -f2-)" true BLOCKED_BEFORE_MUTATION no_live_mutation_attempted
  probe_messages || {
    [[ "$messages_status" == "FAIL" ]] && finalize_fail "messages_access_failed"
    finalize_blocked "messages_access:$(grep '^reason=' "$messages_file" | cut -d= -f2-)" true BLOCKED_BEFORE_MUTATION no_live_mutation_attempted
  }
  if [[ -z "${MORROW_REAL_QA_CHAT_PUBLIC_ID:-}" ]]; then
    finalize_blocked "selected_test_chat:missing_MORROW_REAL_QA_CHAT_PUBLIC_ID" true BLOCKED_BEFORE_MUTATION no_live_mutation_attempted
  fi
  selected_chat_status=PASS
  if [[ "${MORROW_APPROVAL_LIVE_RECEIPT_ALLOW_SURFACE_QA:-false}" != "true" ]]; then
    finalize_blocked "calendar_reminders_access:surface_QA_opt_in_missing" true BLOCKED_BEFORE_MUTATION no_live_mutation_attempted
  fi
  if ! probe_calendar_reminders; then
    finalize_fail "calendar_or_reminders_access_failed"
  fi
  if [[ "$calendar_status" == "BLOCKED" || "$reminders_status" == "BLOCKED" ]]; then
    finalize_blocked "calendar_or_reminders_access_blocked" false PASS lifecycle_surface_cleanup_completed
  fi
  if [[ -n "$surface_fixture_status" ]]; then
    finalize_blocked "calendar_reminders_access:test_fixture_only" false PASS lifecycle_surface_cleanup_completed
  fi
  if [[ "${MORROW_APPROVAL_LIVE_RECEIPT_ALLOW_MUTATION:-false}" != "true" ]]; then
    finalize_blocked "proposal_creation:live_mutation_opt_in_missing" false PASS lifecycle_surface_cleanup_completed
  fi
  finalize_blocked "controlled_approval_rejection_observation:not_implemented_without_manual_proof" false PASS lifecycle_surface_cleanup_completed
}

prepare_out_dir
if [[ -n "$fixture_case" ]]; then
  fixture_receipt
  exit $?
fi
run_live
