#!/usr/bin/env bash
set -euo pipefail

CANARY="MORROW_PRIVACY_CANARY_RAW_TRAJECTORY"
DEFAULT_OUT_DIR=".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke"
PHASE_EVIDENCE_DIR=".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval"
OUT_DIR_MARKER=".morrow-phase5-trajectory-eval-smoke-out-dir"

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

source "$SCRIPT_DIR/messages-calendar-approval-trajectory-smoke-runtime.sh"

original_args=("$@")
out_dir="$DEFAULT_OUT_DIR"
assert_canary_rejection=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out-dir)
      [[ $# -ge 2 ]] || die "--out-dir requires a path"
      out_dir="$2"
      shift 2
      ;;
    --assert-canary-rejection)
      assert_canary_rejection=1
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      usage
      die "unknown argument: $1"
      ;;
  esac
done

require_command cargo
require_command grep
require_command node
require_command npm
require_command sqlite3

repo_root_abs="$(pwd -P)"
out_dir_abs="$(prepare_out_dir "$out_dir")" || exit $?
clear_known_artifacts

invocation="$(quote_invocation "${original_args[@]}")"
trace_out="$out_dir_abs/trace.jsonl"
storage_readback="$out_dir_abs/storage-readback.json"
decision_evidence="$out_dir_abs/decision-evidence.json"
trajectory_report="$out_dir_abs/trajectory-report.json"
privacy_report="$out_dir_abs/privacy-inspect.txt"
canary_report="$out_dir_abs/canary-rejection.txt"
cleanup_receipt="$out_dir_abs/cleanup-receipt.txt"
summary_report="$out_dir_abs/summary.txt"
local_runner_source_rel="$out_dir/local-runner-source"
local_runner_source_abs="$out_dir_abs/local-runner-source/local-trajectory-run.json"

echo "scenario: Phase 5 messages calendar approval trajectory eval smoke"
echo "out_dir: $out_dir"
echo "backend: local tests and synthetic privacy surfaces only"

run_logged "$out_dir_abs/command-logs/cargo-reconcile-trajectory-runner.txt" \
  scripts/run-messages-calendar-approval-trajectory-local-runner.sh \
  --out-dir "$local_runner_source_rel" \
  --assert-live-surface-rejection
run_logged "$out_dir_abs/command-logs/cargo-diagnostics-trajectory-trace.txt" \
  cargo test --manifest-path crates/morrow-diagnostics/Cargo.toml --test messages_calendar_approval_trajectory_trace
run_logged "$out_dir_abs/command-logs/cargo-storage-decision-evidence.txt" \
  cargo test --manifest-path crates/morrow-storage/Cargo.toml --test decision_evidence
run_native_trajectory_test "$out_dir_abs/command-logs/cargo-native-trajectory-eval.txt"
run_logged "$out_dir_abs/command-logs/npm-status-view.txt" npm test -- --run StatusView
run_logged "$out_dir_abs/command-logs/npm-messages-tauri-commands.txt" npm test -- --run messagesTauriCommands
for command_log in "$out_dir_abs"/command-logs/*.txt; do
  sanitize_local_paths "$command_log"
done

node scripts/messages-calendar-approval-trajectory-eval-report.mjs generate \
  --out-dir "$out_dir_abs" \
  --invocation "$invocation" \
  --local-run-source "$local_runner_source_abs"

canary_root="$out_dir_abs/privacy-surface-canary"
prepare_privacy_surface "$canary_root" "yes"
set +e
canary_output="$(scripts/privacy-inspect.sh \
  "$canary_root/morrow.sqlite" \
  "$canary_root/logs" \
  "$canary_root/forbidden-tokens.txt" \
  "$canary_root/diagnostics" 2>&1)"
canary_status=$?
set -e
if [[ "$canary_status" -eq 0 ]]; then
  printf '%s\n' "$canary_output" > "$canary_report"
  die "expected privacy-inspect to reject injected diagnostics canary"
fi
{
  echo "scenario: assert-canary-rejection"
  echo "invocation: scripts/privacy-inspect.sh <synthetic-db> <synthetic-logs> <forbidden-tokens-file> <synthetic-diagnostics-dir>"
  echo "observable: privacy-inspect exited non-zero before sanitization"
  echo "exit_status: $canary_status"
  echo "assert_flag: $assert_canary_rejection"
  echo "result: PASS"
} > "$canary_report"
safe_remove_path "$canary_root"

privacy_root="$out_dir_abs/privacy-surface"
prepare_privacy_surface "$privacy_root" "no"
scripts/privacy-inspect.sh \
  "$privacy_root/morrow.sqlite" \
  "$privacy_root/logs" \
  "$privacy_root/forbidden-tokens.txt" \
  "$privacy_root/diagnostics" \
  > "$privacy_report" 2>&1
safe_remove_path "$privacy_root"

{
  echo "scenario: phase 5 synthetic privacy cleanup"
  echo "invocation: $invocation"
  echo "observable: synthetic privacy surfaces removed after inspection"
  echo "removed: synthetic privacy surface"
  echo "removed: synthetic canary privacy surface"
  echo "result: PASS"
} > "$cleanup_receipt"

{
  echo "scenario: Phase 5 messages calendar approval trajectory eval smoke"
  echo "generated_at_utc: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo "invocation: $invocation"
  echo "backend_observable: no live backend, provider network, EventKit, Messages, Calendar, Reminders, Phoenix, Langfuse, or vendor backend was used; targeted tests use local fixtures/fakes and synthetic privacy surfaces"
  echo "coverage: scheduled_meeting_accepted"
  echo "coverage: scheduled_meeting_rejected"
  echo "coverage: scheduled_meeting_edited_before_approval"
  echo "coverage: task_reminder_accepted"
  echo "coverage: task_reminder_rejected"
  echo "coverage: provider_quiet_low_confidence"
  echo "coverage: collateral_damage_non_target_preserved"
  echo "coverage: replay_idempotent_retry"
  echo "coverage: privacy_canary_rejection"
  echo "trace: trace.jsonl"
  echo "storage_readback: storage-readback.json"
  echo "decision_evidence: decision-evidence.json"
  echo "report: trajectory-report.json"
  echo "privacy_inspection: privacy-inspect.txt"
  echo "canary_rejection: canary-rejection.txt"
  echo "cleanup_receipt: cleanup-receipt.txt"
  echo "command_logs: command-logs/"
  echo "result: PASS"
} > "$summary_report"

for path in "$trace_out" "$storage_readback" "$decision_evidence" "$trajectory_report" "$privacy_report" \
  "$canary_report" "$cleanup_receipt" "$summary_report"; do
  require_file "$path"
  reject_literal_in_file "$CANARY" "$path"
done

node scripts/messages-calendar-approval-trajectory-eval-report.mjs validate --out-dir "$out_dir_abs"

echo "PASS phase 5 messages calendar approval trajectory eval smoke"
echo "summary: $out_dir/summary.txt"
echo "trace: $out_dir/trace.jsonl"
echo "privacy_report: $out_dir/privacy-inspect.txt"
