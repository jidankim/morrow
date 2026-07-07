#!/usr/bin/env bash
set -euo pipefail

DEFAULT_OUT_DIR=".omo/evidence/phase-6-cloud-eval-monitoring/final-smoke"
PHASE_EVIDENCE_DIR=".omo/evidence/phase-6-cloud-eval-monitoring"
RUN_TIMEOUT_SECONDS="${RUN_TIMEOUT_SECONDS:-30}"

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/cloud-eval-monitoring-smoke/command-runtime.sh
source "$SCRIPT_DIR/cloud-eval-monitoring-smoke/command-runtime.sh"

usage() {
  cat >&2 <<'EOF'
usage: run-cloud-eval-monitoring-smoke.sh [--out-dir <path>] [--assert-canary-rejection]

Runs the Phase 6 cloud eval monitoring smoke with local deterministic fixtures only.
EOF
}

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

require_command node

smoke_started_epoch_ms="$(node -e 'process.stdout.write(String(Date.now()))')"
out_dir_abs="$(prepare_out_dir "$out_dir")" || exit $?
out_dir_cli="$(relative_to_repo "$out_dir_abs")"
command_logs="$out_dir_abs/command-logs"
invocation="scripts/run-cloud-eval-monitoring-smoke.sh --out-dir $out_dir_cli"
if [[ "$assert_canary_rejection" -eq 1 ]]; then
  invocation="$invocation --assert-canary-rejection"
fi

healthy_fixture="scripts/cloud-eval-monitoring-fixtures/healthy-events.json"
baseline_fixture="scripts/cloud-eval-monitoring-fixtures/baseline-events.json"
canary_fixture="scripts/cloud-eval-monitoring-fixtures/privacy-canary-events.json"
malformed_fixture="scripts/cloud-eval-monitoring-fixtures/malformed-schema-events.json"
regression_fixture="scripts/cloud-eval-monitoring-fixtures/regression-events.json"
stale_fixture="scripts/cloud-eval-monitoring-fixtures/stale-events.json"

echo "scenario: Phase 6 cloud eval monitoring smoke"
echo "out_dir: $out_dir_cli"
echo "backend: local deterministic fixtures only"

preflight_status=0
set +e
run_logged "$command_logs/preflight.txt" \
  node scripts/cloud-eval-monitoring-preflight.mjs --out-dir "$out_dir_cli"
preflight_status=$?
set -e
if [[ "$preflight_status" -ne 0 ]]; then
  set +e
  node "$SCRIPT_DIR/cloud-eval-monitoring-smoke/dirty-preflight-only.mjs" \
    "$out_dir_abs/preflight-report.json"
  set -e
  die "preflight failed; final smoke requires a PASS preflight report"
fi

run_logged "$command_logs/validate-input.txt" \
  node scripts/cloud-eval-monitoring-report.mjs validate-input --input "$healthy_fixture" --out-dir "$out_dir_cli"
run_logged "$command_logs/generate.txt" \
  node scripts/cloud-eval-monitoring-report.mjs generate --input "$healthy_fixture" --out-dir "$out_dir_cli"
run_logged "$command_logs/metrics.txt" \
  node scripts/cloud-eval-monitoring-report.mjs metrics --input "$healthy_fixture" --baseline "$baseline_fixture" --out-dir "$out_dir_cli"
run_logged "$command_logs/dashboard.txt" \
  node scripts/cloud-eval-monitoring-report.mjs dashboard --input "$healthy_fixture" --baseline "$baseline_fixture" --out-dir "$out_dir_cli"
run_logged "$command_logs/gate.txt" \
  node scripts/cloud-eval-monitoring-report.mjs gate --input "$healthy_fixture" --baseline "$baseline_fixture" --out-dir "$out_dir_cli"

canary_status=0
set +e
run_logged "$command_logs/negative-privacy-canary.txt" \
  node scripts/cloud-eval-monitoring-report.mjs validate-input --input "$canary_fixture" --out-dir "$out_dir_cli/negative/privacy-canary"
canary_status=$?
set -e
if [[ "$canary_status" -eq 0 ]]; then
  die "privacy canary rejection was not proven"
fi

malformed_status=0
set +e
run_logged "$command_logs/negative-malformed-input.txt" \
  node scripts/cloud-eval-monitoring-report.mjs validate-input --input "$malformed_fixture" --out-dir "$out_dir_cli/negative/malformed-input"
malformed_status=$?
set -e
[[ "$malformed_status" -ne 0 ]] || die "malformed input negative case unexpectedly passed"

stale_status=0
set +e
run_logged "$command_logs/negative-stale-state.txt" \
  node scripts/cloud-eval-monitoring-report.mjs validate-input --input "$stale_fixture" --out-dir "$out_dir_cli/negative/stale-state"
stale_status=$?
set -e
[[ "$stale_status" -ne 0 ]] || die "stale-state negative case unexpectedly passed"

regression_status=0
set +e
run_logged "$command_logs/negative-regression-gate.txt" \
  node scripts/cloud-eval-monitoring-report.mjs gate --input "$regression_fixture" --baseline "$baseline_fixture" --out-dir "$out_dir_cli/negative/regression-gate"
regression_status=$?
set -e
[[ "$regression_status" -ne 0 ]] || die "regression gate negative case unexpectedly passed"

cancel_resume_status=0
repeated_interruptions_first_status=0
repeated_interruptions_second_status=0
run_adversarial_interruption_probe cancel_resume_status "cancel-resume"
run_adversarial_interruption_probe repeated_interruptions_first_status "repeated-interruptions-1"
run_adversarial_interruption_probe repeated_interruptions_second_status "repeated-interruptions-2"

run_logged "$command_logs/deterministic-repeat-gate.txt" \
  node scripts/cloud-eval-monitoring-report.mjs gate --input "$healthy_fixture" --baseline "$baseline_fixture" --out-dir "$out_dir_cli/repeat-gate"

node "$SCRIPT_DIR/cloud-eval-monitoring-smoke/artifact-finalizer.mjs" \
  "$out_dir_abs" \
  "$smoke_started_epoch_ms" \
  "$canary_status" \
  "$malformed_status" \
  "$stale_status" \
  "$regression_status" \
  "$preflight_status" \
  "$invocation" \
  "$cancel_resume_status" \
  "$repeated_interruptions_first_status" \
  "$repeated_interruptions_second_status"

require_non_empty_file "$out_dir_abs/privacy-inspect.txt"
require_non_empty_file "$out_dir_abs/summary.txt"

echo "PASS phase 6 cloud eval monitoring smoke"
echo "summary: $out_dir_cli/summary.txt"
echo "release_gate: $out_dir_cli/release-gate.json"
