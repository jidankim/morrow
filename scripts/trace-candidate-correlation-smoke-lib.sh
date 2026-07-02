#!/usr/bin/env bash

CANARY="MORROW_PRIVACY_CANARY_RAW_TEXT"

usage() {
  cat >&2 <<'EOF'
usage: run-trace-candidate-correlation-smoke.sh --out-dir <path> [--assert-canary-rejection]

Runs the Phase 2 trace candidate correlation smoke with local fake fixtures only.
EOF
}

die() {
  echo "error: $*" >&2
  exit 64
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
}

display_path() {
  local path_abs="$1"
  case "$path_abs" in
    "$repo_root_abs")
      printf '.'
      ;;
    "$repo_root_abs"/*)
      printf '%s' "${path_abs#"$repo_root_abs"/}"
      ;;
    *)
      printf '[external-evidence-dir]/%s' "$(basename "$path_abs")"
      ;;
  esac
}

sanitize_artifact() {
  local input="$1"
  local output="$2"
  node - "$input" "$output" "$repo_root_abs" <<'NODE'
const fs = require("node:fs")

const [input, output, repoRoot] = process.argv.slice(2)
const escapeRegExp = (value) => value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
let content = fs.readFileSync(input, "utf8")

content = content.replace(new RegExp(escapeRegExp(repoRoot), "g"), "<workspace>")
content = content.replace(/\/Users\/[^\s"'`)]+/g, "<local-path>")
content = content.replace(/\/private\/[^\s"'`)]+/g, "<local-path>")
content = content.replace(/\/var\/folders\/[^\s"'`)]+/g, "<local-path>")

fs.writeFileSync(output, content)
NODE
}

finalize_command_log() {
  local status="$1"
  local raw_path="$2"
  local clean_path="$3"
  local label="$4"
  sanitize_artifact "$raw_path" "$clean_path"
  rm -f "$raw_path"
  if [[ "$status" -ne 0 ]]; then
    die "$label failed; see $(display_path "$clean_path")"
  fi
}

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
INSERT INTO evidence (id, excerpt_hash) VALUES ('phase-2-correlation-smoke', 'sha256:synthetic');
INSERT INTO quiet_logs (id, reason_code) VALUES ('phase-2-quiet', 'confidence_below_threshold');
SQL
  printf 'phase 2 trace candidate correlation smoke sanitized log\n' > "$logs_dir/morrow.log"
  printf '{"scenario":"phase-2-trace-candidate-correlation","source":"local-fake-fixtures"}\n' \
    > "$diagnostics_dir/traces/trace.jsonl"
  cp "$decision_report" "$diagnostics_dir/exports/decision-evidence.json"
  cp "$summary_report" "$diagnostics_dir/exports/summary.txt"
  printf '{"scenario":"phase-2-trace-candidate-correlation","latestEvalStatus":"never_run"}\n' \
    > "$diagnostics_dir/evals/summary.json"
  if [[ "$include_canary" == "yes" ]]; then
    printf '{"leak":"%s"}\n' "$CANARY" > "$diagnostics_dir/exports/canary-report.json"
  fi
  printf '%s\n' "$CANARY" > "$root/forbidden-tokens.txt"
}
