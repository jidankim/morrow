usage() {
  cat >&2 <<'EOF'
usage: run-human-approval-correction-smoke.sh [--out-dir <path>] [--assert-canary-rejection]

Runs the Phase 4 human approval correction smoke with local targeted tests only.
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
  local quoted="scripts/run-human-approval-correction-smoke.sh"
  for arg in "$@"; do
    printf -v quoted '%s %q' "$quoted" "$arg"
  done
  printf '%s' "$quoted"
}

canonicalize_output_path() {
  local candidate="$1"
  local absolute
  local probe
  local suffix=""
  local base
  local probe_abs

  [[ -n "$candidate" ]] || die "--out-dir must not be empty"
  case "/$candidate/" in
    *"/../"*|*"/./"*) die "--out-dir must not contain . or .. path components: $candidate" ;;
  esac

  if [[ "$candidate" == /* ]]; then
    absolute="$candidate"
  else
    absolute="$repo_root_abs/$candidate"
  fi

  case "$absolute" in
    "/") die "--out-dir must be a dedicated evidence or tmp directory, not /" ;;
  esac

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
  local phase_evidence_abs
  local prepared_abs
  local phase_child

  phase_evidence_abs="$(canonicalize_output_path "$PHASE_EVIDENCE_DIR")" || exit $?
  prepared_abs="$(canonicalize_output_path "$requested")" || exit $?

  case "$prepared_abs" in
    /|"$repo_root_abs"|"$phase_evidence_abs")
      die "--out-dir must be a dedicated evidence directory, not $prepared_abs"
      ;;
  esac

  case "$prepared_abs" in
    "$phase_evidence_abs"/*)
      phase_child="${prepared_abs#"$phase_evidence_abs"/}"
      case "$phase_child" in
        "") die "--out-dir must be a dedicated smoke directory under $PHASE_EVIDENCE_DIR: $requested" ;;
        *-smoke) ;;
        *) die "--out-dir must be a dedicated smoke directory under $PHASE_EVIDENCE_DIR: $requested" ;;
      esac
      ;;
    *)
      die "--out-dir cleanup is only allowed under $PHASE_EVIDENCE_DIR: $requested"
      ;;
  esac

  mkdir -p "$prepared_abs"
  printf 'phase 4 human approval correction smoke out-dir\n' > "$prepared_abs/$OUT_DIR_MARKER"
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
    trace.jsonl storage-readback.json decision-evidence.json phase4-correction-decision-evidence.json
    human-approval-correction-report.json privacy-inspect.txt canary-rejection.txt cleanup-receipt.txt
    summary.txt command-log-pass-counts.txt
  )
  for artifact in "${known_artifacts[@]}"; do
    rm -f "$out_dir_abs/$artifact"
  done
  safe_remove_path "$out_dir_abs/command-logs"
  safe_remove_path "$out_dir_abs/privacy-surface"
  safe_remove_path "$out_dir_abs/privacy-surface-canary"
  mkdir -p "$out_dir_abs/command-logs"
}

run_native_phase4_test() {
  local log_path="$1"
  if command -v xcrun >/dev/null 2>&1; then
    local macos_sdk
    macos_sdk="$(xcrun --sdk macosx --show-sdk-path)"
    {
      echo "scenario: targeted native phase 4 decision evidence test"
      echo "invocation: MORROW_PHASE4_DECISION_EVIDENCE_DIR=<out-dir> cargo test --manifest-path src-tauri/Cargo.toml --test native_scan_codex decision_evidence -- --nocapture"
      SDKROOT="$macos_sdk" LIBRARY_PATH="$macos_sdk/usr/lib" \
        MORROW_PHASE4_DECISION_EVIDENCE_DIR="$out_dir_abs" \
        cargo test --manifest-path src-tauri/Cargo.toml --test native_scan_codex decision_evidence -- --nocapture
    } > "$log_path" 2>&1
  else
    run_logged "$log_path" env MORROW_PHASE4_DECISION_EVIDENCE_DIR="$out_dir_abs" \
      cargo test --manifest-path src-tauri/Cargo.toml --test native_scan_codex decision_evidence -- --nocapture
  fi
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
INSERT INTO evidence (id, excerpt_hash) VALUES ('phase4-human-approval-correction-smoke', 'sha256:synthetic');
INSERT INTO quiet_logs (id, reason_code) VALUES ('phase4-correction-run', 'user_correction_applied');
SQL
  printf 'phase 4 human approval correction smoke sanitized log\n' > "$logs_dir/morrow.log"
  cp "$trace_out" "$diagnostics_dir/traces/trace.jsonl"
  cp "$correction_report" "$diagnostics_dir/evals/human-approval-correction-report.json"
  cp "$storage_readback" "$diagnostics_dir/exports/storage-readback.json"
  cp "$decision_evidence" "$diagnostics_dir/exports/decision-evidence.json"
  if [[ "$include_canary" == "yes" ]]; then
    cp crates/morrow-diagnostics/fixtures/human_approval_correction_trace_canary_rejected.jsonl \
      "$diagnostics_dir/traces/canary.jsonl"
  fi
  printf '%s\n' "$CANARY" > "$root/forbidden-tokens.txt"
}
