#!/usr/bin/env bash
set -euo pipefail

CANARY="MORROW_PRIVACY_CANARY_RAW_TEXT"
DEFAULT_OUT_DIR=".omo/evidence/phase-1-production-provider-trace-sink/final-smoke"

usage() {
  cat >&2 <<'EOF'
usage: run-production-trace-sink-smoke.sh [--out-dir <path>] [--assert-canary-rejection]

Runs the Phase 1 production provider trace sink smoke with local fake Codex fixtures only.
EOF
}

die() {
  echo "error: $*" >&2
  exit 64
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
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
require_command cargo
require_command sqlite3
require_command grep

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

reject_literal_in_file() {
  local literal="$1"
  local path="$2"
  if grep -Fq "$literal" "$path"; then
    die "forbidden literal leaked into $path"
  fi
}

reject_forbidden_trace_content() {
  local path="$1"
  for literal in \
    "$CANARY" \
    "Maybe meet tomorrow?" \
    "Provider meeting" \
    "iMessage;-;+15555550103" \
    "beta-provider-route" \
    '"prompt"' \
    '"response"' \
    '"raw_text"' \
    '"raw_json"' \
    '"provider_json"' \
    '"embedding"' \
    '"full_message"'; do
    reject_literal_in_file "$literal" "$path"
  done
}

prepare_privacy_surface() {
  local root="$1"
  local include_canary="$2"
  local db_path="$root/morrow.sqlite"
  local logs_dir="$root/logs"
  local diagnostics_dir="$root/diagnostics"

  mkdir -p "$logs_dir" "$diagnostics_dir/traces" "$diagnostics_dir/evals" "$diagnostics_dir/exports"
  sqlite3 "$db_path" <<'SQL'
CREATE TABLE evidence (id TEXT PRIMARY KEY, excerpt_hash TEXT);
CREATE TABLE quiet_logs (id TEXT PRIMARY KEY, reason_code TEXT);
INSERT INTO evidence (id, excerpt_hash) VALUES ('phase-1-smoke', 'sha256:synthetic');
INSERT INTO quiet_logs (id, reason_code) VALUES ('phase-1-quiet', 'provider_fixture_smoke');
SQL
  printf 'phase 1 production trace sink smoke sanitized log\n' > "$logs_dir/morrow.log"
  cp "$trace_out" "$diagnostics_dir/traces/trace.jsonl"
  printf '{"scenario":"phase-1-production-provider-trace-sink","source":"local-fake-codex-fixture"}\n' \
    > "$diagnostics_dir/evals/summary.json"
  printf '{"source":"morrow-local-production-trace-jsonl","records":1}\n' \
    > "$diagnostics_dir/exports/local-viewer-payload.json"
  if [[ "$include_canary" == "yes" ]]; then
    printf '{"leak":"%s"}\n' "$CANARY" > "$diagnostics_dir/traces/canary.jsonl"
  fi
  printf '%s\n' "$CANARY" > "$root/forbidden-tokens.txt"
}

known_artifacts=(
  "trace.jsonl"
  "privacy-inspect.txt"
  "summary.txt"
  "cleanup-receipt.txt"
  "cargo-test.txt"
  "privacy-canary-rejection.txt"
)
for artifact in "${known_artifacts[@]}"; do
  rm -f "$out_dir_abs/$artifact"
done
safe_remove_path "$out_dir_abs/privacy-surface"
safe_remove_path "$out_dir_abs/privacy-surface-canary"

trace_out="$out_dir_abs/trace.jsonl"
privacy_report="$out_dir_abs/privacy-inspect.txt"
summary_report="$out_dir_abs/summary.txt"
cleanup_receipt="$out_dir_abs/cleanup-receipt.txt"
cargo_report="$out_dir_abs/cargo-test.txt"
canary_report="$out_dir_abs/privacy-canary-rejection.txt"

echo "scenario: phase 1 production provider trace sink smoke"
echo "repo: $REPO_ROOT"
echo "out_dir: $out_dir_abs"
echo "backend: local fake Codex fixture only"
echo "command: cargo test production_scan_writes_local_diagnostics_trace"

if command -v xcrun >/dev/null 2>&1; then
  macos_sdk="$(xcrun --sdk macosx --show-sdk-path)"
  SDKROOT="$macos_sdk" LIBRARY_PATH="$macos_sdk/usr/lib" \
    MORROW_TASK4_TRACE_COPY="$trace_out" \
    cargo test -p morrow \
      production_scan_writes_local_diagnostics_trace -- --nocapture \
      > "$cargo_report" 2>&1
else
  MORROW_TASK4_TRACE_COPY="$trace_out" \
    cargo test -p morrow \
      production_scan_writes_local_diagnostics_trace -- --nocapture \
      > "$cargo_report" 2>&1
fi

require_file "$cargo_report"
require_file "$trace_out"
reject_forbidden_trace_content "$trace_out"

if ! grep -Fq '"component":"provider"' "$trace_out"; then
  die "trace JSONL did not include provider component spans"
fi
if ! grep -Fq '"operation":"provider_result"' "$trace_out"; then
  die "trace JSONL did not include provider result span"
fi

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

require_file "$privacy_report"
reject_literal_in_file "$CANARY" "$privacy_report"
reject_forbidden_trace_content "$trace_out"

invocation="scripts/run-production-trace-sink-smoke.sh --out-dir $out_dir_abs"
if [[ "$assert_canary_rejection" -eq 1 ]]; then
  invocation="$invocation --assert-canary-rejection"
fi

{
  echo "scenario: diagnostics cleanup receipt"
  echo "invocation: $invocation"
  echo "observable: synthetic app-data diagnostics privacy surface removed after inspection"
  echo "removed: $privacy_root"
  if [[ "$assert_canary_rejection" -eq 1 ]]; then
    echo "removed: $out_dir_abs/privacy-surface-canary"
  fi
  echo "result: PASS"
} > "$cleanup_receipt"

{
  echo "scenario: Phase 1 production provider trace sink smoke"
  echo "generated_at_utc: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo "invocation: $invocation"
  echo "provider_fixture: src-tauri/tests/native_scan_codex/trace_sink.rs::production_scan_writes_local_diagnostics_trace"
  echo "diagnostics: diagnostics were enabled for the smoke fixture"
  echo "trace_observable: traces were written to $trace_out"
  echo "backend_observable: no live provider, no live vendor, and no live backend was required; cargo test uses RecordingCodexRunner fake output and does not call live Codex/OpenAI/network/vendor backends"
  echo "trace: $trace_out"
  echo "privacy_inspection: $privacy_report"
  echo "cargo_test: $cargo_report"
  echo "cleanup_receipt: $cleanup_receipt"
  if [[ "$assert_canary_rejection" -eq 1 ]]; then
    echo "canary rejection: expected and verified by --assert-canary-rejection; artifact: $canary_report"
  else
    echo "canary rejection: not requested; rerun with --assert-canary-rejection to verify expected rejection"
  fi
  echo "result: PASS"
} > "$summary_report"

require_file "$summary_report"
require_file "$cleanup_receipt"
reject_literal_in_file "$CANARY" "$summary_report"
reject_literal_in_file "$CANARY" "$cleanup_receipt"

echo "PASS phase 1 production provider trace sink smoke"
echo "trace: $trace_out"
echo "privacy_report: $privacy_report"
echo "summary: $summary_report"
echo "cleanup_receipt: $cleanup_receipt"
