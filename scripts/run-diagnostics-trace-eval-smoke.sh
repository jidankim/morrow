#!/usr/bin/env bash
set -euo pipefail

# SIZE_OK: Todo 12 requires one smoke entry point that captures trace, eval, export,
# privacy-inspect, canary rejection, and cleanup evidence in a single auditable flow.
CANARY="MORROW_PRIVACY_CANARY_RAW_TEXT"
DEFAULT_OUT_DIR=".omo/evidence/task-12-final-flow"

usage() {
  cat >&2 <<'EOF'
usage: run-diagnostics-trace-eval-smoke.sh [--out-dir <path>] [--assert-canary-rejection]

Runs the local Morrow diagnostics trace/eval smoke flow without live vendor backends.
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
mkdir -p "$out_dir"
out_dir_abs="$(cd "$out_dir" && pwd -P)"
repo_root_abs="$(pwd -P)"

case "$out_dir_abs" in
  /|"$repo_root_abs")
    die "--out-dir must be a dedicated evidence directory, not $out_dir_abs"
    ;;
esac

safe_remove_path() {
  local target="$1"
  [[ -n "$target" ]] || die "refusing to remove an empty path"
  case "$target" in
    "$out_dir_abs"/*)
      rm -rf "$target"
      ;;
    *)
      die "refusing to remove path outside --out-dir: $target"
      ;;
  esac
}

require_file() {
  local path="$1"
  [[ -s "$path" ]] || die "expected non-empty artifact: $path"
}

reject_canary_in_file() {
  local path="$1"
  if grep -Fq "$CANARY" "$path"; then
    die "privacy canary leaked into $path"
  fi
}

assert_no_forbidden_export_fields() {
  local path="$1"
  node - "$path" <<'NODE'
const fs = require("fs");
const path = process.argv[2];
const forbidden = new Set([
  "raw_text",
  "prompt",
  "response",
  "raw_json",
  "embedding",
  "provider_json",
  "full_message",
  "raw_title",
  "title_text",
  "full_title",
  "unredacted_title",
]);
const value = JSON.parse(fs.readFileSync(path, "utf8"));
const hits = [];
function visit(node, trail) {
  if (Array.isArray(node)) {
    node.forEach((item, index) => visit(item, `${trail}[${index}]`));
    return;
  }
  if (node && typeof node === "object") {
    for (const [key, child] of Object.entries(node)) {
      if (forbidden.has(key)) hits.push(`${trail}.${key}`);
      visit(child, trail ? `${trail}.${key}` : key);
    }
  }
}
visit(value, "");
if (hits.length > 0) {
  console.error(`forbidden export fields in ${path}: ${hits.join(", ")}`);
  process.exit(1);
}
NODE
}

assert_eval_summary_fields() {
  local path="$1"
  node - "$path" <<'NODE'
const fs = require("fs");
const path = process.argv[2];
const summary = JSON.parse(fs.readFileSync(path, "utf8"));
function requireValue(condition, message) {
  if (!condition) {
    console.error(message);
    process.exit(1);
  }
}
requireValue(summary.dataset_family, "missing dataset_family");
requireValue(summary.dataset_version, "missing dataset_version");
requireValue(summary.dataset_provenance && summary.dataset_provenance.fixture_path, "missing dataset_provenance");
requireValue(summary.trace_schema_version, "missing trace_schema_version");
requireValue(summary.provider_identity && summary.provider_identity.provider_id, "missing provider_identity.provider_id");
requireValue(summary.provider_identity && summary.provider_identity.model_id, "missing provider_identity.model_id");
requireValue(summary.provider_identity && summary.provider_identity.template_version, "missing provider_identity.template_version");
requireValue(summary.trace_identity && summary.trace_identity.prompt_identity, "missing trace_identity.prompt_identity");
requireValue(summary.canary_scan_result && summary.canary_scan_result.passed === true, "canary scan did not pass");
NODE
}

assert_exports_downstream() {
  local trace_path="$1"
  local phoenix_path="$2"
  local langfuse_path="$3"
  node - "$trace_path" "$phoenix_path" "$langfuse_path" <<'NODE'
const fs = require("fs");
const [tracePath, phoenixPath, langfusePath] = process.argv.slice(2);
const traceLines = fs.readFileSync(tracePath, "utf8").split(/\n/).filter(Boolean);
const traceIds = new Set(traceLines.map((line) => JSON.parse(line).trace.trace_id));
const phoenix = JSON.parse(fs.readFileSync(phoenixPath, "utf8"));
const langfuse = JSON.parse(fs.readFileSync(langfusePath, "utf8"));
if (phoenix.source?.source_of_truth !== "morrow_local_trace_jsonl") {
  console.error("Phoenix payload is not marked downstream of local trace JSONL");
  process.exit(1);
}
if (langfuse.source !== "morrow-local-trace-jsonl" || langfuse.ownership !== "morrow_local_traces_are_canonical") {
  console.error("Langfuse payload is not marked downstream of local trace JSONL");
  process.exit(1);
}
const phoenixIds = new Set((phoenix.spans || []).map((span) => span.trace_id));
const langfuseIds = new Set((langfuse.traces || []).map((trace) => trace.id));
for (const id of phoenixIds) {
  if (!traceIds.has(id)) {
    console.error(`Phoenix payload contains unknown trace id ${id}`);
    process.exit(1);
  }
}
for (const id of langfuseIds) {
  if (!traceIds.has(id)) {
    console.error(`Langfuse payload contains unknown trace id ${id}`);
    process.exit(1);
  }
}
if (phoenixIds.size === 0 || langfuseIds.size === 0) {
  console.error("viewer/importer payloads are empty");
  process.exit(1);
}
NODE
}

prepare_synthetic_privacy_surface() {
  local root="$1"
  local db_path="$2"
  local logs_dir="$3"
  local diagnostics_dir="$4"
  local include_canary="$5"

  mkdir -p "$logs_dir" "$diagnostics_dir/traces" "$diagnostics_dir/evals" "$diagnostics_dir/exports"
  sqlite3 "$db_path" <<'SQL'
CREATE TABLE evidence (id TEXT PRIMARY KEY, excerpt_hash TEXT);
CREATE TABLE quiet_logs (id TEXT PRIMARY KEY, reason_code TEXT);
INSERT INTO evidence (id, excerpt_hash) VALUES ('evidence-1', 'sha256:synthetic');
INSERT INTO quiet_logs (id, reason_code) VALUES ('quiet-1', 'no_schedule_signal');
SQL
  printf 'synthetic sanitized local log\n' > "$logs_dir/morrow.log"
  cp "$trace_out" "$diagnostics_dir/traces/trace.jsonl"
  cp "$eval_summary" "$diagnostics_dir/evals/eval-summary.json"
  cp "$phoenix_payload" "$diagnostics_dir/exports/phoenix-payload.json"
  cp "$langfuse_payload" "$diagnostics_dir/exports/langfuse-payload.json"
  if [[ "$include_canary" == "yes" ]]; then
    printf '{"leak":"%s"}\n' "$CANARY" > "$diagnostics_dir/traces/canary.jsonl"
  fi
  printf '%s\n' "$CANARY" > "$root/forbidden-tokens.txt"
}

known_artifacts=(
  "trace.jsonl"
  "eval-summary.json"
  "eval-summary.raw.json"
  "phoenix-payload.json"
  "langfuse-payload.json"
  "privacy-inspect.txt"
  "delete-all.txt"
  "privacy-canary-rejection.txt"
  "forbidden-tokens.txt"
)
for artifact in "${known_artifacts[@]}"; do
  rm -f "$out_dir_abs/$artifact"
done
safe_remove_path "$out_dir_abs/synthetic"
safe_remove_path "$out_dir_abs/synthetic-canary"

fixtures="crates/morrow-detection/fixtures/golden_conversations.json"
[[ -s "$fixtures" ]] || die "missing fixture dataset: $fixtures"

trace_out="$out_dir_abs/trace.jsonl"
eval_summary="$out_dir_abs/eval-summary.json"
eval_summary_raw="$out_dir_abs/eval-summary.raw.json"
phoenix_payload="$out_dir_abs/phoenix-payload.json"
langfuse_payload="$out_dir_abs/langfuse-payload.json"
privacy_report="$out_dir_abs/privacy-inspect.txt"
delete_all_report="$out_dir_abs/delete-all.txt"

echo "scenario: local diagnostics trace/eval smoke"
echo "repo: $REPO_ROOT"
echo "out_dir: $out_dir_abs"
echo "fixtures: $fixtures"

echo "command: cargo run detection_eval"
cargo run --manifest-path crates/morrow-detection/Cargo.toml --example detection_eval -- \
  --fixtures "$fixtures" \
  --out "$eval_summary_raw" \
  --trace-out "$trace_out"

node - "$eval_summary_raw" "$eval_summary" "$fixtures" <<'NODE'
const fs = require("fs");
const [rawPath, outPath, fixtures] = process.argv.slice(2);
const summary = JSON.parse(fs.readFileSync(rawPath, "utf8"));
summary.dataset_provenance = {
  source: "existing Morrow golden conversations fixture",
  fixture_path: fixtures,
  first_party: true,
  fixture_priority: ["morrow-golden", "morrow-adversarial"],
  third_party_corpus_required: false
};
summary.trace_identity = {
  trace_schema_version: summary.trace_schema_version,
  provider_id: summary.provider_identity?.provider_id,
  model_id: summary.provider_identity?.model_id,
  prompt_identity: summary.provider_identity?.template_version
};
fs.writeFileSync(outPath, `${JSON.stringify(summary, null, 2)}\n`);
NODE
rm -f "$eval_summary_raw"

echo "command: cargo run export_phoenix"
cargo run --manifest-path crates/morrow-diagnostics/Cargo.toml --example export_phoenix -- \
  --input "$trace_out" \
  --out "$phoenix_payload"

echo "command: cargo run export_langfuse"
cargo run --manifest-path crates/morrow-diagnostics/Cargo.toml --example export_langfuse -- \
  --input "$trace_out" \
  --out "$langfuse_payload"

if [[ "$assert_canary_rejection" -eq 1 ]]; then
  canary_root="$out_dir_abs/synthetic-canary"
  canary_db="$canary_root/morrow.sqlite"
  canary_logs="$canary_root/logs"
  canary_diagnostics="$canary_root/diagnostics"
  mkdir -p "$canary_root"
  prepare_synthetic_privacy_surface "$canary_root" "$canary_db" "$canary_logs" "$canary_diagnostics" "yes"
  set +e
  canary_output="$(scripts/privacy-inspect.sh "$canary_db" "$canary_logs" "$canary_root/forbidden-tokens.txt" "$canary_diagnostics" 2>&1)"
  canary_status=$?
  set -e
  if [[ "$canary_status" -eq 0 ]]; then
    echo "$canary_output" > "$out_dir_abs/privacy-canary-rejection.txt"
    die "expected privacy-inspect to reject injected diagnostics canary"
  fi
  {
    echo "scenario: assert-canary-rejection"
    echo "invocation: scripts/privacy-inspect.sh <synthetic-db> <synthetic-logs> <forbidden-tokens-file> <synthetic-diagnostics-dir>"
    echo "observable: privacy-inspect exited non-zero before sanitization"
    echo "exit_status: $canary_status"
    echo "result: PASS"
  } > "$out_dir_abs/privacy-canary-rejection.txt"
  echo "PASS canary rejection: privacy-inspect exited $canary_status before sanitization"
  safe_remove_path "$canary_root"
fi

synthetic_root="$out_dir_abs/synthetic"
synthetic_db="$synthetic_root/morrow.sqlite"
synthetic_logs="$synthetic_root/logs"
synthetic_diagnostics="$synthetic_root/diagnostics"
mkdir -p "$synthetic_root"
prepare_synthetic_privacy_surface "$synthetic_root" "$synthetic_db" "$synthetic_logs" "$synthetic_diagnostics" "no"
cp "$synthetic_root/forbidden-tokens.txt" "$out_dir_abs/forbidden-tokens.txt"

echo "command: scripts/privacy-inspect.sh"
scripts/privacy-inspect.sh "$synthetic_db" "$synthetic_logs" "$synthetic_root/forbidden-tokens.txt" "$synthetic_diagnostics" > "$privacy_report" 2>&1
safe_remove_path "$synthetic_root"
rm -f "$out_dir_abs/forbidden-tokens.txt"

echo "command: cargo test delete_all_removes_diagnostics_artifacts"
{
  echo "scenario: repository Delete All synthetic diagnostics cleanup"
  echo "invocation: cargo test --manifest-path src-tauri/Cargo.toml delete_all_removes_diagnostics_artifacts -- --nocapture"
  if command -v xcrun >/dev/null 2>&1; then
    macos_sdk="$(xcrun --sdk macosx --show-sdk-path)"
    SDKROOT="$macos_sdk" LIBRARY_PATH="$macos_sdk/usr/lib" \
      cargo test --manifest-path src-tauri/Cargo.toml delete_all_removes_diagnostics_artifacts -- --nocapture
  else
    cargo test --manifest-path src-tauri/Cargo.toml delete_all_removes_diagnostics_artifacts -- --nocapture
  fi
} > "$delete_all_report" 2>&1

for path in "$trace_out" "$eval_summary" "$phoenix_payload" "$langfuse_payload" "$privacy_report" "$delete_all_report"; do
  require_file "$path"
  reject_canary_in_file "$path"
done

assert_eval_summary_fields "$eval_summary"
assert_exports_downstream "$trace_out" "$phoenix_payload" "$langfuse_payload"
assert_no_forbidden_export_fields "$phoenix_payload"
assert_no_forbidden_export_fields "$langfuse_payload"

echo "PASS diagnostics trace/eval smoke"
echo "trace: $trace_out"
echo "eval_summary: $eval_summary"
echo "phoenix_payload: $phoenix_payload"
echo "langfuse_payload: $langfuse_payload"
echo "privacy_report: $privacy_report"
echo "delete_all_report: $delete_all_report"
