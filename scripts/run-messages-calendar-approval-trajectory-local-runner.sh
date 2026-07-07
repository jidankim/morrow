#!/usr/bin/env bash
set -euo pipefail

out_dir=".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/local-runner"
assert_live_surface_rejection=0
repo_root="$(git rev-parse --show-toplevel)"
phase_root_rel=".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval"
phase_root_abs="$(cd "$repo_root/$phase_root_rel" && pwd -P)"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out-dir)
      if [[ $# -lt 2 ]]; then
        echo "missing value for --out-dir" >&2
        exit 2
      fi
      out_dir="$2"
      shift 2
      ;;
    --assert-live-surface-rejection)
      assert_live_surface_rejection=1
      shift
      ;;
    --help|-h)
      echo "usage: $0 [--out-dir DIR] [--assert-live-surface-rejection]"
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

resolve_out_dir() {
  local raw="$1"
  local candidate
  case "$raw" in
    /*) candidate="$raw" ;;
    *) candidate="$repo_root/$raw" ;;
  esac
  local parent
  parent="$(dirname "$candidate")"
  if [[ ! -d "$parent" ]]; then
    echo "--out-dir parent does not exist: $raw" >&2
    exit 2
  fi
  local abs
  abs="$(cd "$parent" && pwd -P)/$(basename "$candidate")"
  case "$abs" in
    "$phase_root_abs"/local-runner*) printf '%s\n' "$abs" ;;
    "$phase_root_abs"/*-smoke/local-runner*) printf '%s\n' "$abs" ;;
    *)
      echo "--out-dir must be a local-runner* directory under $phase_root_rel" >&2
      exit 2
      ;;
  esac
}

out_dir="$(resolve_out_dir "$out_dir")"

MORROW_PHASE5_LOCAL_RUNNER_OUT_DIR="$out_dir" \
  cargo test -p morrow-reconcile --test messages_calendar_approval_trajectory runner -- --nocapture

test -s "$out_dir/local-trajectory-run.json"

if [[ "$assert_live_surface_rejection" -eq 1 ]]; then
  test -s "$out_dir/live-surface-rejection.json"
  grep -Fq '"attempted": true' "$out_dir/live-surface-rejection.json"
  grep -Fq '"blocked_before_side_effect": true' "$out_dir/live-surface-rejection.json"
  grep -Fq '"sanitized_target": "[REDACTED_LOCAL_PATH]"' "$out_dir/live-surface-rejection.json"
fi
