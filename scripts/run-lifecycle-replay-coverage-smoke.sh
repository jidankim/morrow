#!/usr/bin/env bash
set -euo pipefail

CANARY="MORROW_PRIVACY_CANARY_RAW_TEXT"
DEFAULT_OUT_DIR=".omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke"
PHASE_EVIDENCE_DIR=".omo/evidence/phase-3-lifecycle-replay-coverage"
OUT_DIR_MARKER=".morrow-lifecycle-replay-coverage-smoke-out-dir"

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/lifecycle-replay-smoke-guards.sh"
source "$SCRIPT_DIR/lifecycle-replay-smoke-commands.sh"
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

[[ -n "$out_dir" ]] || die "--out-dir must not be empty"
require_command cargo
require_command grep
require_command node
require_command sqlite3

repo_root_abs="$(pwd -P)"
out_dir_abs="$(prepare_out_dir "$out_dir")" || exit $?
clear_known_artifacts

invocation="$(quote_invocation "${original_args[@]}")"
trace_out="$out_dir_abs/trace.jsonl"
storage_readback="$out_dir_abs/storage-readback.json"
storage_readback_db="$out_dir_abs/storage-readback.sqlite"
storage_readback_source="$out_dir_abs/storage-readback-source.json"
lifecycle_report="$out_dir_abs/lifecycle-report.json"
privacy_report="$out_dir_abs/privacy-inspect.txt"
canary_report="$out_dir_abs/canary-rejection.txt"
cleanup_receipt="$out_dir_abs/cleanup-receipt.txt"
summary_report="$out_dir_abs/summary.txt"

echo "scenario: Phase 3 lifecycle replay coverage smoke"
echo "out_dir: $out_dir_abs"
echo "backend: local tests and synthetic privacy surfaces only"

run_logged "$out_dir_abs/command-logs/cargo-storage-lifecycle-test.txt" \
  cargo test -p morrow-storage lifecycle
run_logged "$out_dir_abs/command-logs/cargo-reconcile-lifecycle-test.txt" \
  cargo test -p morrow-reconcile lifecycle
run_logged "$out_dir_abs/command-logs/cargo-reconcile-validation-test.txt" \
  cargo test -p morrow-reconcile --test validation
run_logged "$out_dir_abs/command-logs/cargo-diagnostics-lifecycle-trace-test.txt" \
  cargo test -p morrow-diagnostics lifecycle_trace
run_native_test "$out_dir_abs/command-logs/cargo-native-proposal-replay-test.txt" proposal_replay
run_native_test "$out_dir_abs/command-logs/cargo-native-lifecycle-replay-test.txt" replay_modes
run_logged "$out_dir_abs/command-logs/reconcile-smoke.txt" \
  cargo run -p morrow-reconcile --example reconcile_smoke -- --scenario lifecycle-suite
run_logged "$out_dir_abs/command-logs/storage-replay-smoke.txt" \
  cargo run -p morrow-storage --example replay_smoke -- --db "$storage_readback_db"
run_storage_readback_query \
  "$out_dir_abs/command-logs/storage-readback-sqlite-query.txt" \
  "$storage_readback_db" \
  "$storage_readback_source"

node scripts/lifecycle-replay-smoke-report.mjs generate \
  --out-dir "$out_dir_abs" \
  --invocation "$invocation" \
  --storage-readback-source "$storage_readback_source"

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
  echo "scenario: lifecycle replay synthetic privacy cleanup"
  echo "invocation: $invocation"
  echo "observable: synthetic privacy surfaces removed after inspection"
  echo "removed: synthetic privacy surface"
  echo "removed: synthetic canary privacy surface"
  echo "result: PASS"
} > "$cleanup_receipt"

{
  echo "scenario: Phase 3 lifecycle replay coverage smoke"
  echo "generated_at_utc: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo "invocation: $invocation"
  echo "backend_observable: no live backend, no live vendor, and no live network was required; targeted cargo tests use local fixtures/fakes and synthetic privacy surfaces"
  echo "coverage: superseded=candidate_superseded"
  echo "coverage: rescheduled=candidate_rescheduled"
  echo "coverage: cancelled=candidate_cancelled"
  echo "coverage: dry-run=calendar_dry_run"
  echo "coverage: commit-idempotent=calendar_commit_idempotency"
  echo "coverage: replay-run=replay_run"
  echo "trace: trace.jsonl"
  echo "storage_readback: storage-readback.json"
  echo "storage_readback_source: storage-readback-source.json"
  echo "storage_readback_db: storage-readback.sqlite"
  echo "lifecycle_report: lifecycle-report.json"
  echo "privacy_inspection: privacy-inspect.txt"
  echo "canary_rejection: canary-rejection.txt"
  echo "cleanup_receipt: cleanup-receipt.txt"
  echo "command_logs: command-logs/"
  echo "result: PASS"
} > "$summary_report"

for path in "$trace_out" "$storage_readback" "$storage_readback_source" "$lifecycle_report" "$privacy_report" \
  "$canary_report" "$cleanup_receipt" "$summary_report"; do
  require_file "$path"
  reject_literal_in_file "$CANARY" "$path"
done

node scripts/lifecycle-replay-smoke-report.mjs validate --out-dir "$out_dir_abs"

echo "PASS phase 3 lifecycle replay coverage smoke"
echo "summary: $summary_report"
echo "trace: $trace_out"
echo "privacy_report: $privacy_report"
