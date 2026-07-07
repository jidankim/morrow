#!/usr/bin/env bash
set -euo pipefail

DEFAULT_OUT_DIR=".omo/evidence/phase-7-wider-product-rollout/final-smoke"
PHASE_EVIDENCE_DIR=".omo/evidence/phase-7-wider-product-rollout"
RUN_TIMEOUT_SECONDS="${RUN_TIMEOUT_SECONDS:-45}"

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/cloud-eval-monitoring-smoke/command-runtime.sh"

usage() {
  cat >&2 <<'EOF'
usage: run-wider-product-rollout-smoke.sh [--out-dir <path>] [--fixture <manifest>] [--assert-canary-rejection]

Runs the Phase 7 wider product rollout final smoke with local deterministic fixtures only.
EOF
}

out_dir="$DEFAULT_OUT_DIR"
manifest_fixture="docs/wider-product-rollout-manifest.json"
assert_canary_rejection=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out-dir)
      [[ $# -ge 2 ]] || die "--out-dir requires a path"
      out_dir="$2"
      shift 2
      ;;
    --fixture)
      [[ $# -ge 2 ]] || die "--fixture requires a manifest path"
      manifest_fixture="$2"
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
require_command sqlite3

run_phase7_logged() {
  local log_path="$1"
  shift
  local command_status=0
  local restore_errexit=0
  case "$-" in
    *e*) restore_errexit=1 ;;
  esac
  mkdir -p "$(dirname -- "$log_path")"
  set +e
  {
    echo "scenario: phase 7 wider product rollout smoke command"
    echo "timeout_seconds: $RUN_TIMEOUT_SECONDS"
    echo "invocation: $(quote_command "$@")"
    node "$SCRIPT_DIR/cloud-eval-monitoring-smoke/run-with-timeout.mjs" "$RUN_TIMEOUT_SECONDS" "$@"
    command_status=$?
    echo "exit_status: $command_status"
  } > "$log_path" 2>&1
  if [[ "$restore_errexit" -eq 1 ]]; then
    set -e
  fi
  sanitize_local_paths "$log_path"
  return "$command_status"
}

smoke_started_epoch_ms="$(node -e 'process.stdout.write(String(Date.now()))')"
out_dir_abs="$(prepare_out_dir "$out_dir")" || exit $?
out_dir_cli="$(relative_to_repo "$out_dir_abs")"
command_logs="$out_dir_abs/command-logs"
invocation="scripts/run-wider-product-rollout-smoke.sh --out-dir $out_dir_cli"
if [[ "$manifest_fixture" != "docs/wider-product-rollout-manifest.json" ]]; then
  invocation="$invocation --fixture $manifest_fixture"
fi
if [[ "$assert_canary_rejection" -eq 1 ]]; then
  invocation="$invocation --assert-canary-rejection"
fi

echo "scenario: Phase 7 wider product rollout final smoke"
echo "out_dir: $out_dir_cli"
echo "backend: local deterministic fixtures only"
echo "deployment_action=none"

preflight_status=0
manifest_status=0
beta_status=0
docs_status=0
retention_status=0
runbook_status=0
regression_status=0
kill_switch_status=0
privacy_status=0
canary_status=0
default_blocked_status=0
missing_artifacts_status=0

set +e
run_phase7_logged "$command_logs/preflight.txt" \
  node scripts/wider-product-rollout-preflight.mjs --out-dir "$out_dir_cli/preflight"
preflight_status=$?
run_phase7_logged "$command_logs/manifest-validation.txt" \
  node scripts/wider-product-rollout-report.mjs validate-manifest --manifest "$manifest_fixture" --out-dir "$out_dir_cli/manifest"
manifest_status=$?
run_phase7_logged "$command_logs/beta-gate.txt" \
  node scripts/wider-product-rollout-beta-gate.mjs --bundle-dir src-tauri/target/release/bundle --out-dir "$out_dir_cli/beta"
beta_status=$?
run_phase7_logged "$command_logs/docs-qa.txt" \
  node scripts/wider-product-rollout-docs-qa.mjs docs/wider-product-rollout.md docs/privacy.md README.md docs/beta-testing.md "$out_dir_cli/docs-qa.md"
docs_status=$?
run_phase7_logged "$command_logs/retention-qa.txt" \
  node scripts/wider-product-rollout-retention-qa.mjs --out-dir "$out_dir_cli/retention"
retention_status=$?
run_phase7_logged "$command_logs/runbook-qa.txt" \
  node scripts/wider-product-rollout-runbook-qa.mjs docs/operations/wider-product-rollout-runbook.md "$manifest_fixture" "$out_dir_cli/runbook-qa.md"
runbook_status=$?
run_phase7_logged "$command_logs/regression-gate.txt" \
  node scripts/wider-product-rollout-report.mjs gate --manifest "$manifest_fixture" --phase6-dir .omo/evidence/phase-6-cloud-eval-monitoring/final-smoke --beta-dir "$out_dir_cli/beta" --retention-dir "$out_dir_cli/retention" --docs-qa "$out_dir_cli/docs-qa.md" --runbook-qa "$out_dir_cli/runbook-qa.md" --out-dir "$out_dir_cli/regression"
regression_status=$?
run_phase7_logged "$command_logs/kill-switch-check.txt" \
  node scripts/wider-product-rollout-report.mjs kill-switch-check --manifest "$manifest_fixture" --out-dir "$out_dir_cli/kill-switch"
kill_switch_status=$?
run_phase7_logged "$command_logs/negative-privacy-canary.txt" \
  node scripts/wider-product-rollout-report.mjs privacy-scan --path scripts/wider-product-rollout-fixtures/privacy-canary-report.json --out-dir "$out_dir_cli/negative/privacy-canary"
canary_status=$?
if [[ "$assert_canary_rejection" -eq 1 && "$canary_status" -eq 0 ]]; then
  set -e
  die "privacy canary rejection was not proven"
fi
if [[ "${PHASE7_SMOKE_SKIP_NESTED_NEGATIVE:-0}" -eq 1 ]]; then
  default_blocked_status=1
  printf 'scenario: nested negative probe skipped\nexit_status: 1\n' > "$command_logs/negative-default-available-blocked-beta.txt"
else
  run_phase7_logged "$command_logs/negative-default-available-blocked-beta.txt" \
    env PHASE7_SMOKE_SKIP_NESTED_NEGATIVE=1 bash scripts/run-wider-product-rollout-smoke.sh --out-dir "$out_dir_cli/negative/default-available-blocked-beta" --fixture scripts/wider-product-rollout-fixtures/default-available-with-blocked-beta.json
  default_blocked_status=$?
fi
run_phase7_logged "$command_logs/negative-missing-artifacts.txt" \
  node scripts/wider-product-rollout-report.mjs gate --manifest "$manifest_fixture" --phase6-dir "$out_dir_cli/negative/missing-phase6" --beta-dir "$out_dir_cli/beta" --retention-dir "$out_dir_cli/retention" --docs-qa "$out_dir_cli/docs-qa.md" --runbook-qa "$out_dir_cli/runbook-qa.md" --out-dir "$out_dir_cli/negative/missing-artifacts"
missing_artifacts_status=$?
set -e

printf '%s\n%s\n' "phase7-forbidden-token-alpha" "phase7-forbidden-private-marker-beta" > "$out_dir_abs/forbidden-tokens.txt"
privacy_db="$out_dir_abs/privacy-fixture.sqlite"
sqlite3 "$privacy_db" \
  "CREATE TABLE evidence (id TEXT PRIMARY KEY, summary TEXT); CREATE TABLE quiet_logs (id TEXT PRIMARY KEY, status TEXT); INSERT INTO evidence VALUES ('phase7-smoke','sanitized metadata only'); INSERT INTO quiet_logs VALUES ('local','pass');"
mkdir -p "$out_dir_abs/privacy-diagnostics/traces" "$out_dir_abs/privacy-diagnostics/evals" "$out_dir_abs/privacy-diagnostics/exports"
printf '{"trace_id":"phase7-smoke","privacy_tier":"metadata_only"}\n' > "$out_dir_abs/privacy-diagnostics/traces/trace.jsonl"
printf '{"metric":"metadata_only"}\n' > "$out_dir_abs/privacy-diagnostics/evals/eval.jsonl"
printf '{"export":"metadata_only"}\n' > "$out_dir_abs/privacy-diagnostics/exports/export.json"

set +e
run_phase7_logged "$command_logs/privacy-inspect-command.txt" \
  scripts/privacy-inspect.sh "$privacy_db" "$command_logs" "$out_dir_abs/forbidden-tokens.txt" "$out_dir_abs/privacy-diagnostics"
privacy_status=$?
set -e
cp "$command_logs/privacy-inspect-command.txt" "$out_dir_abs/privacy-inspect.txt"
if [[ "$privacy_status" -eq 0 ]]; then
  printf 'result: PASS\n' >> "$out_dir_abs/privacy-inspect.txt"
fi
rm -f "$privacy_db"

node scripts/wider-product-rollout/smoke-finalizer.mjs \
  "$out_dir_abs" \
  "$smoke_started_epoch_ms" \
  "$invocation" \
  "$preflight_status" \
  "$manifest_status" \
  "$beta_status" \
  "$docs_status" \
  "$retention_status" \
  "$runbook_status" \
  "$regression_status" \
  "$kill_switch_status" \
  "$privacy_status" \
  "$canary_status" \
  "$default_blocked_status" \
  "$missing_artifacts_status"

require_non_empty_file "$out_dir_abs/privacy-inspect.txt"
require_non_empty_file "$out_dir_abs/summary.txt"
require_non_empty_file "$out_dir_abs/negative-matrix.json"

echo "PASS phase 7 wider product rollout smoke"
echo "summary: $out_dir_cli/summary.txt"
echo "regression_gate: $out_dir_cli/regression-gate.json"
