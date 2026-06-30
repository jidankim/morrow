#!/usr/bin/env bash
set -euo pipefail

DEFAULT_OUT_DIR=".omo/evidence/task-8-feedback-eval-smoke"

usage() {
  cat >&2 <<'EOF'
usage: run-feedback-eval-smoke.sh [--out-dir <path>]

Runs the local Morrow feedback/eval DB smoke flow without private Messages data.
EOF
}

die() {
  echo "error: $*" >&2
  exit 64
}

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

out_dir="$DEFAULT_OUT_DIR"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out-dir)
      [[ $# -ge 2 ]] || die "--out-dir requires a path"
      out_dir="$2"
      shift 2
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
mkdir -p "$out_dir"

workflow_db="/tmp/morrow-feedback-e2e.sqlite"
delete_db="/tmp/morrow-feedback-e2e-delete.sqlite"
metrics_report="/tmp/morrow-feedback-e2e-metrics.json"
eval_report="/tmp/morrow-feedback-e2e-metrics.feedback-eval.json"
e2e_output="$out_dir/mvp-e2e-output.txt"
delete_failure_output="$out_dir/delete-all-failure-output.txt"
summary="$out_dir/feedback-eval-smoke-summary.txt"

rm -f "$workflow_db" "$delete_db" "$metrics_report" "$eval_report"

MORROW_E2E_DB="$workflow_db" \
MORROW_E2E_DELETE_DB="$delete_db" \
MORROW_E2E_METRICS_REPORT="$metrics_report" \
  cargo run --manifest-path crates/morrow-reconcile/Cargo.toml --example mvp_e2e \
  > "$e2e_output" 2>&1

cargo test --manifest-path crates/morrow-storage/Cargo.toml \
  feedback_eval_invalid_delete_all_preserves_tables \
  > "$delete_failure_output" 2>&1

for required in \
  "PASS mvp_e2e" \
  "privacy_dataset_ready=true" \
  "feedback_eval_ready=true" \
  "labels_recorded=" \
  "snapshots_recorded=" \
  "eval_report=$eval_report"; do
  grep -Fq "$required" "$e2e_output" || die "missing E2E output line: $required"
done

grep -Fq "test feedback_eval_invalid_delete_all_preserves_tables ... ok" \
  "$delete_failure_output" || die "delete-all preservation test did not pass"

[[ -s "$metrics_report" ]] || die "missing metrics report: $metrics_report"
[[ -s "$eval_report" ]] || die "missing eval report: $eval_report"

node - "$eval_report" <<'NODE'
const fs = require("fs");
const path = process.argv[2];
const report = JSON.parse(fs.readFileSync(path, "utf8"));
for (const key of ["schema_version", "eval_run_id", "status", "cases_evaluated", "cases_skipped"]) {
  if (!Object.prototype.hasOwnProperty.call(report, key)) {
    throw new Error(`missing eval report key: ${key}`);
  }
}
if (report.cases_evaluated + report.cases_skipped < 3) {
  throw new Error("expected at least three feedback eval cases");
}
NODE

{
  echo "scenario=feedback-eval-db-smoke"
  echo "mvp_e2e_output=$e2e_output"
  echo "delete_failure_output=$delete_failure_output"
  echo "metrics_report=$metrics_report"
  echo "eval_report=$eval_report"
  echo "result=PASS"
} > "$summary"

echo "PASS feedback eval smoke"
echo "summary=$summary"
