#!/usr/bin/env bash
set -euo pipefail

CANARY="MORROW_PRIVACY_CANARY_RAW_CORRECTION"
DEFAULT_OUT_DIR=".omo/evidence/phase-4-human-approval-correction/final-smoke"
PHASE_EVIDENCE_DIR=".omo/evidence/phase-4-human-approval-correction"
OUT_DIR_MARKER=".morrow-human-approval-correction-smoke-out-dir"

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/lifecycle-replay-smoke-commands.sh"
source "$SCRIPT_DIR/human-approval-correction-smoke-lib.sh"
cd "$REPO_ROOT"

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
correction_report="$out_dir_abs/human-approval-correction-report.json"
privacy_report="$out_dir_abs/privacy-inspect.txt"
canary_report="$out_dir_abs/canary-rejection.txt"
cleanup_receipt="$out_dir_abs/cleanup-receipt.txt"
summary_report="$out_dir_abs/summary.txt"

echo "scenario: Phase 4 human approval correction smoke"
echo "out_dir: $out_dir_abs"
echo "backend: local tests and synthetic privacy surfaces only"

run_logged "$out_dir_abs/command-logs/cargo-diagnostics-human-approval-correction-trace.txt" \
  cargo test -p morrow-diagnostics --test human_approval_correction_trace
run_logged "$out_dir_abs/command-logs/cargo-reconcile-human-approval-correction.txt" \
  cargo test -p morrow-reconcile --test human_approval_correction -- --nocapture
run_logged "$out_dir_abs/command-logs/cargo-storage-decision-evidence.txt" \
  cargo test -p morrow-storage --test decision_evidence
run_logged "$out_dir_abs/command-logs/cargo-storage-decision-evidence-privacy.txt" \
  cargo test -p morrow-storage --test decision_evidence_privacy
run_native_phase4_test "$out_dir_abs/command-logs/cargo-native-decision-evidence.txt"
run_logged "$out_dir_abs/command-logs/npm-status-view.txt" npm test -- --run StatusView
run_logged "$out_dir_abs/command-logs/npm-messages-tauri-commands.txt" npm test -- --run messagesTauriCommands

mv "$out_dir_abs/phase4-correction-decision-evidence.json" "$decision_evidence"
node scripts/human-approval-correction-smoke-report.mjs generate \
  --out-dir "$out_dir_abs" \
  --invocation "$invocation" \
  --decision-evidence-source "$decision_evidence"

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
  echo "scenario: phase 4 synthetic privacy cleanup"
  echo "invocation: $invocation"
  echo "observable: synthetic privacy surfaces removed after inspection"
  echo "removed: synthetic privacy surface"
  echo "removed: synthetic canary privacy surface"
  echo "result: PASS"
} > "$cleanup_receipt"

{
  echo "scenario: Phase 4 human approval correction smoke"
  echo "generated_at_utc: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo "invocation: $invocation"
  echo "backend_observable: no live network, vendor, Messages, Calendar, or Reminders access was required; targeted tests use local fixtures/fakes and synthetic privacy surfaces"
  echo "coverage: user_correction=user_corrected"
  echo "coverage: proposal_outcome=accepted"
  echo "coverage: proposal_outcome=rejected_observed"
  echo "coverage: proposal_outcome=pending_edited"
  echo "coverage: proposal_outcome=unknown"
  echo "coverage: field_quality=title_edited"
  echo "coverage: trace_retention=retained"
  echo "trace: trace.jsonl"
  echo "storage_readback: storage-readback.json"
  echo "decision_evidence: decision-evidence.json"
  echo "report: human-approval-correction-report.json"
  echo "privacy_inspection: privacy-inspect.txt"
  echo "canary_rejection: canary-rejection.txt"
  echo "cleanup_receipt: cleanup-receipt.txt"
  echo "command_logs: command-logs/"
  echo "result: PASS"
} > "$summary_report"

for path in "$trace_out" "$storage_readback" "$decision_evidence" "$correction_report" "$privacy_report" \
  "$canary_report" "$cleanup_receipt" "$summary_report"; do
  require_file "$path"
  reject_literal_in_file "$CANARY" "$path"
done

node scripts/human-approval-correction-smoke-report.mjs validate --out-dir "$out_dir_abs"

echo "PASS phase 4 human approval correction smoke"
echo "summary: $summary_report"
echo "trace: $trace_out"
echo "privacy_report: $privacy_report"
