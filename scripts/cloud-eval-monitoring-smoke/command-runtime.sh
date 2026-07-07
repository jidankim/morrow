#!/usr/bin/env bash

REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"
repo_root_abs="$(pwd -P)"
out_dir_abs="${out_dir_abs-}"
command_logs="${command_logs-}"

die() {
  echo "error: $*" >&2
  exit 64
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
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

relative_to_repo() {
  node -e 'const path = require("node:path"); const rel = path.relative(process.argv[1], process.argv[2]); process.stdout.write(rel && !rel.startsWith("..") && !path.isAbsolute(rel) ? rel : process.argv[2]);' "$repo_root_abs" "$1"
}

prepare_out_dir() {
  local requested="$1"
  local phase_evidence_abs prepared_abs
  phase_evidence_abs="$(canonicalize_output_path "$PHASE_EVIDENCE_DIR")" || exit $?
  prepared_abs="$(canonicalize_output_path "$requested")" || exit $?
  case "$prepared_abs" in
    "$phase_evidence_abs"/*) ;;
    *) die "--out-dir must be under $PHASE_EVIDENCE_DIR: $requested" ;;
  esac
  mkdir -p "$prepared_abs/command-logs"
  printf '%s' "$prepared_abs"
}

quote_command() {
  local quoted=""
  local arg
  for arg in "$@"; do
    if [[ -z "$quoted" ]]; then
      printf -v quoted '%q' "$arg"
    else
      printf -v quoted '%s %q' "$quoted" "$arg"
    fi
  done
  printf '%s' "$quoted"
}

sanitize_local_paths() {
  local file_path="$1"
  node -e '
const fs = require("node:fs");
const filePath = process.argv[1];
const repoRoot = process.argv[2];
const text = fs.readFileSync(filePath, "utf8");
const sanitized = text
  .split(repoRoot).join("[REPO_ROOT]")
  .replaceAll("/Users/", "[USERS]/")
  .replaceAll("/private/", "[PRIVATE]/")
  .replaceAll("/var/folders/", "[VAR_FOLDERS]/");
fs.writeFileSync(filePath, sanitized);
' "$file_path" "$repo_root_abs"
}

run_logged() {
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
    echo "scenario: phase 6 cloud eval monitoring smoke command"
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

require_non_empty_file() {
  local artifact="$1"
  [[ -s "$artifact" ]] || die "expected non-empty artifact: $artifact"
}

run_adversarial_interruption_probe() {
  local status_var="$1"
  local label="$2"
  local receipt="$out_dir_abs/adversarial/$label.receipt.txt"
  local token="phase6_${label}_$$"
  local probe_status=0
  local saved_timeout="$RUN_TIMEOUT_SECONDS"
  RUN_TIMEOUT_SECONDS=1
  set +e
  run_logged "$command_logs/adversarial-$label.txt" \
    bash scripts/cloud-eval-monitoring-smoke/interruption-probe-child.sh "$receipt" "$label" "$token"
  probe_status=$?
  RUN_TIMEOUT_SECONDS="$saved_timeout"
  set -e
  [[ "$probe_status" -ne 0 ]] || die "interruption probe unexpectedly exited zero: $label"
  require_non_empty_file "$receipt"
  printf -v "$status_var" '%s' "$probe_status"
}
