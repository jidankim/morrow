#!/usr/bin/env bash

usage() {
  cat >&2 <<'EOF'
usage: run-messages-calendar-approval-trajectory-eval-smoke.sh [--out-dir <path>] [--assert-canary-rejection]

Runs the Phase 5 messages-calendar approval trajectory eval smoke with local targeted tests only.
EOF
}

die() {
  echo "error: $*" >&2
  exit 64
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
}

quote_invocation() {
  local quoted="scripts/run-messages-calendar-approval-trajectory-eval-smoke.sh"
  for arg in "$@"; do
    printf -v quoted '%s %q' "$quoted" "$arg"
  done
  printf '%s' "$quoted"
}

canonicalize_output_path() {
  local candidate="$1"
  local absolute probe suffix="" base probe_abs
  [[ -n "$candidate" ]] || die "--out-dir must not be empty"
  case "/$candidate/" in
    *"/../"*|*"/./"*) die "--out-dir must not contain . or .. path components: $candidate" ;;
  esac
  if [[ "$candidate" == /* ]]; then
    absolute="$candidate"
  else
    absolute="$repo_root_abs/$candidate"
  fi
  [[ "$absolute" != "/" ]] || die "--out-dir must be a dedicated evidence directory, not /"
  while [[ "$absolute" != "/" && "$absolute" == */ ]]; do
    absolute="${absolute%/}"
  done
  probe="$absolute"
  while [[ ! -e "$probe" ]]; do
    base="$(basename -- "$probe")"
    [[ -n "$base" && "$base" != "/" && "$base" != "." ]] || die "invalid --out-dir: $candidate"
    suffix="/$base$suffix"
    probe="$(dirname -- "$probe")"
  done
  [[ -d "$probe" ]] || die "--out-dir parent is not a directory: $candidate"
  probe_abs="$(cd -- "$probe" && pwd -P)"
  printf '%s%s' "$probe_abs" "$suffix"
}

prepare_out_dir() {
  local requested="$1"
  local phase_evidence_abs prepared_abs phase_child
  phase_evidence_abs="$(canonicalize_output_path "$PHASE_EVIDENCE_DIR")" || exit $?
  prepared_abs="$(canonicalize_output_path "$requested")" || exit $?
  case "$prepared_abs" in
    /|"$repo_root_abs"|"$phase_evidence_abs")
      die "--out-dir must be a dedicated smoke directory, not $prepared_abs"
      ;;
  esac
  case "$prepared_abs" in
    "$phase_evidence_abs"/*)
      phase_child="${prepared_abs#"$phase_evidence_abs"/}"
      case "$phase_child" in
        *-smoke) ;;
        *) die "--out-dir must be a dedicated smoke directory under $PHASE_EVIDENCE_DIR: $requested" ;;
      esac
      ;;
    *) die "--out-dir cleanup is only allowed under $PHASE_EVIDENCE_DIR: $requested" ;;
  esac
  mkdir -p "$prepared_abs"
  printf 'phase 5 messages calendar approval trajectory eval smoke out-dir\n' > "$prepared_abs/$OUT_DIR_MARKER"
  cd -- "$prepared_abs" && pwd -P
}

safe_remove_path() {
  local target="$1"
  [[ -n "$target" ]] || die "refusing to remove an empty path"
  case "$target" in
    "$out_dir_abs"/*) rm -rf "$target" ;;
    *) die "refusing to remove path outside --out-dir: $target" ;;
  esac
}

run_logged() {
  local log_path="$1"
  shift
  {
    echo "scenario: targeted command"
    echo "invocation: $*"
    "$@"
  } > "$log_path" 2>&1
}

sanitize_local_paths() {
  local file_path="$1"
  node -e '
const fs = require("node:fs");
const filePath = process.argv[1];
const repoRoot = process.argv[2];
const text = fs.readFileSync(filePath, "utf8");
const sanitized = text.split(repoRoot).join("[REPO_ROOT]").split("/private/").join("[PRIVATE]/");
fs.writeFileSync(filePath, sanitized);
' "$file_path" "$repo_root_abs"
}

run_native_trajectory_test() {
  local log_path="$1"
  if command -v xcrun >/dev/null 2>&1; then
    local macos_sdk
    macos_sdk="$(xcrun --sdk macosx --show-sdk-path)"
    {
      echo "scenario: targeted native trajectory eval test"
      echo "invocation: cargo test -p morrow --test native_scan_codex trajectory_eval -- --nocapture"
      SDKROOT="$macos_sdk" LIBRARY_PATH="$macos_sdk/usr/lib" \
        cargo test -p morrow --test native_scan_codex trajectory_eval -- --nocapture
    } > "$log_path" 2>&1
  else
    run_logged "$log_path" cargo test -p morrow --test native_scan_codex trajectory_eval -- --nocapture
  fi
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

clear_known_artifacts() {
  local artifact
  local known_artifacts=(
    trace.jsonl storage-readback.json decision-evidence.json trajectory-report.json
    privacy-inspect.txt canary-rejection.txt cleanup-receipt.txt summary.txt
    command-log-pass-counts.txt
  )
  for artifact in "${known_artifacts[@]}"; do
    rm -f "$out_dir_abs/$artifact"
  done
  safe_remove_path "$out_dir_abs/command-logs"
  safe_remove_path "$out_dir_abs/privacy-surface"
  safe_remove_path "$out_dir_abs/privacy-surface-canary"
  safe_remove_path "$out_dir_abs/local-runner-source"
  mkdir -p "$out_dir_abs/command-logs"
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
INSERT INTO evidence (id, excerpt_hash) VALUES ('phase5-trajectory-smoke', 'sha256:synthetic');
INSERT INTO quiet_logs (id, reason_code) VALUES ('phase5-trajectory-run', 'phase5_local_trajectory_eval');
SQL
  printf 'phase 5 trajectory eval smoke sanitized log\n' > "$logs_dir/morrow.log"
  cp "$trace_out" "$diagnostics_dir/traces/trace.jsonl"
  cp "$trajectory_report" "$diagnostics_dir/evals/trajectory-report.json"
  cp "$storage_readback" "$diagnostics_dir/exports/storage-readback.json"
  cp "$decision_evidence" "$diagnostics_dir/exports/decision-evidence.json"
  if [[ "$include_canary" == "yes" ]]; then
    printf '{"leak":"%s"}\n' "$CANARY" > "$diagnostics_dir/traces/canary.jsonl"
  fi
  printf '%s\n' "$CANARY" > "$root/forbidden-tokens.txt"
}
