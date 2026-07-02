#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/trace-candidate-correlation-smoke-lib.sh"
cd "$REPO_ROOT"

out_dir=""
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

[[ -n "$out_dir" ]] || die "--out-dir is required"
require_command cargo
require_command grep
require_command node
require_command npm
require_command sqlite3

mkdir -p "$out_dir"
out_dir_abs="$(cd "$out_dir" && pwd -P)"
repo_root_abs="$(pwd -P)"

case "$out_dir_abs" in
  /|"$repo_root_abs")
    die "--out-dir must be a dedicated evidence directory, not $out_dir_abs"
    ;;
esac
out_dir_display="$(display_path "$out_dir_abs")"

known_artifacts=(
  "summary.txt"
  "decision-evidence.json"
  "privacy-inspect.txt"
  "canary-rejection.txt"
  "cargo-decision-evidence.txt"
  "cargo-decision-evidence.raw.txt"
  "npm-decision-evidence.txt"
  "npm-decision-evidence.raw.txt"
  "cleanup-receipt.txt"
)
for artifact in "${known_artifacts[@]}"; do
  rm -f "$out_dir_abs/$artifact"
done
safe_remove_path "$out_dir_abs/native-reports"
safe_remove_path "$out_dir_abs/privacy-surface"
safe_remove_path "$out_dir_abs/privacy-surface-canary"

native_report_dir="$out_dir_abs/native-reports"
decision_report="$out_dir_abs/decision-evidence.json"
summary_report="$out_dir_abs/summary.txt"
privacy_report="$out_dir_abs/privacy-inspect.txt"
canary_report="$out_dir_abs/canary-rejection.txt"
cargo_report="$out_dir_abs/cargo-decision-evidence.txt"
npm_report="$out_dir_abs/npm-decision-evidence.txt"
cleanup_receipt="$out_dir_abs/cleanup-receipt.txt"
cargo_report_raw="$out_dir_abs/cargo-decision-evidence.raw.txt"
npm_report_raw="$out_dir_abs/npm-decision-evidence.raw.txt"
trap 'rm -f "$cargo_report_raw" "$npm_report_raw"' EXIT
mkdir -p "$native_report_dir"

echo "scenario: phase 2 trace candidate correlation smoke"
echo "repo: workspace"
echo "out_dir: $out_dir_display"
echo "backend: local fake Codex fixtures only"

echo "command: cargo test --manifest-path src-tauri/Cargo.toml --test native_scan_codex decision_evidence -- --nocapture"
set +e
if command -v xcrun >/dev/null 2>&1; then
  macos_sdk="$(xcrun --sdk macosx --show-sdk-path)"
  SDKROOT="$macos_sdk" LIBRARY_PATH="$macos_sdk/usr/lib" \
    MORROW_TASK2_DECISION_EVIDENCE_COPY="$native_report_dir/provider-candidate-retained.json" \
    MORROW_TASK5_DECISION_EVIDENCE_DIR="$native_report_dir" \
    cargo test --manifest-path src-tauri/Cargo.toml --test native_scan_codex decision_evidence -- --nocapture \
    > "$cargo_report_raw" 2>&1
  cargo_status=$?
else
  MORROW_TASK2_DECISION_EVIDENCE_COPY="$native_report_dir/provider-candidate-retained.json" \
    MORROW_TASK5_DECISION_EVIDENCE_DIR="$native_report_dir" \
    cargo test --manifest-path src-tauri/Cargo.toml --test native_scan_codex decision_evidence -- --nocapture \
    > "$cargo_report_raw" 2>&1
  cargo_status=$?
fi
set -e
finalize_command_log "$cargo_status" "$cargo_report_raw" "$cargo_report" "cargo decision evidence"

echo "command: npm test -- --run messagesTauriCommands useSyncScheduler StatusView App.syncResultCounts"
set +e
npm test -- --run messagesTauriCommands useSyncScheduler StatusView App.syncResultCounts \
  > "$npm_report_raw" 2>&1
npm_status=$?
set -e
finalize_command_log "$npm_status" "$npm_report_raw" "$npm_report" "npm decision evidence"

for report in \
  "$cargo_report" \
  "$npm_report" \
  "$native_report_dir/provider-candidate-retained.json" \
  "$native_report_dir/quiet-provider-rejection.json" \
  "$native_report_dir/deleted-diagnostics-root.json" \
  "$native_report_dir/sink-conflict.json"; do
  require_file "$report"
done
node scripts/trace-candidate-correlation-report.mjs assert-clean \
  "$cargo_report" \
  "$npm_report" \
  "$native_report_dir/provider-candidate-retained.json" \
  "$native_report_dir/quiet-provider-rejection.json" \
  "$native_report_dir/deleted-diagnostics-root.json" \
  "$native_report_dir/sink-conflict.json"

node scripts/trace-candidate-correlation-report.mjs build \
  "$native_report_dir/provider-candidate-retained.json" \
  "$native_report_dir/quiet-provider-rejection.json" \
  "$native_report_dir/deleted-diagnostics-root.json" \
  "$native_report_dir/sink-conflict.json" \
  "$decision_report"
require_file "$decision_report"

invocation="scripts/run-trace-candidate-correlation-smoke.sh --out-dir $out_dir_display"
if [[ "$assert_canary_rejection" -eq 1 ]]; then
  invocation="$invocation --assert-canary-rejection"
fi

{
  echo "scenario: Phase 2 trace candidate correlation smoke"
  echo "generated_at_utc: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo "invocation: $invocation"
  echo "provider_candidate_coverage: provider_candidate fixture produced a candidate summary with current local label detection_route=provider_candidate"
  echo "quiet_provider_rejection_coverage: provider_rejection fixture produced a quiet summary with current local label proposal_outcome=unknown"
  echo "retained_trace_coverage: provider candidate and quiet summaries include retained sanitized trace sequences"
  echo "not_retained_deleted_trace_coverage: deleted diagnostics fixture reports diagnosticsMissing without failing correlation"
  echo "backend_observable: no live Codex, OpenAI, network, Messages, Calendar, Phoenix, Langfuse, LangSmith, Braintrust, LiteLLM, Helicone, or cloud backend was required"
  echo "fixture_observable: native smoke used local fake Codex fixtures and frontend smoke used jsdom/vitest mocks only"
  echo "phase4_approval_correction_gap: approve/reject/edit are represented only by closest current local labels; Phase 4 still owns human approval/correction lifecycle labels"
  echo "decision_evidence: decision-evidence.json"
  echo "privacy_inspection: privacy-inspect.txt"
  echo "native_test_log: cargo-decision-evidence.txt"
  echo "frontend_test_log: npm-decision-evidence.txt"
  echo "cleanup_receipt: cleanup-receipt.txt"
  if [[ "$assert_canary_rejection" -eq 1 ]]; then
    echo "canary_rejection: expected and verified before sanitization; artifact: canary-rejection.txt"
  else
    echo "canary_rejection: not requested; rerun with --assert-canary-rejection to verify expected rejection"
  fi
  echo "result: PASS"
} > "$summary_report"
require_file "$summary_report"
node scripts/trace-candidate-correlation-report.mjs assert-clean "$summary_report"

if [[ "$assert_canary_rejection" -eq 1 ]]; then
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
    echo "result: PASS"
  } > "$canary_report"
  safe_remove_path "$canary_root"
fi

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
  echo "scenario: diagnostics cleanup receipt"
  echo "invocation: $invocation"
  echo "observable: synthetic diagnostics/report privacy surfaces removed after inspection"
  echo "removed: privacy-surface"
  if [[ "$assert_canary_rejection" -eq 1 ]]; then
    echo "removed: privacy-surface-canary"
  fi
  echo "result: PASS"
} > "$cleanup_receipt"

require_file "$privacy_report"
require_file "$cleanup_receipt"
node scripts/trace-candidate-correlation-report.mjs assert-clean "$privacy_report" "$cleanup_receipt"
if [[ "$assert_canary_rejection" -eq 1 ]]; then
  require_file "$canary_report"
  node scripts/trace-candidate-correlation-report.mjs assert-clean "$canary_report"
fi

echo "PASS phase 2 trace candidate correlation smoke"
echo "decision_evidence: $out_dir_display/decision-evidence.json"
echo "privacy_report: $out_dir_display/privacy-inspect.txt"
echo "summary: $out_dir_display/summary.txt"
echo "cleanup_receipt: $out_dir_display/cleanup-receipt.txt"
if [[ "$assert_canary_rejection" -eq 1 ]]; then
  echo "canary_rejection: $out_dir_display/canary-rejection.txt"
fi
