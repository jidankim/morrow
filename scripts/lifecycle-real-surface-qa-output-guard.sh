#!/usr/bin/env bash

qa_refuse_unsafe_out_dir() {
  local root="$1"
  local output="$2"

  case "$output" in
    ""|"/"|"$root"|"$root/") qa_fail_usage "refusing unsafe --out-dir: $output" ;;
  esac
}

qa_canonicalize_out_dir() {
  local root="$1"
  local output="$2"
  local absolute
  local probe
  local suffix=""
  local base
  local probe_abs

  qa_refuse_unsafe_out_dir "$root" "$output"
  case "/$output/" in
    *"/../"*|*"/./"*) qa_fail_usage "refusing --out-dir with . or .. path components: $output" ;;
  esac

  if [[ "$output" == /* ]]; then
    absolute="$output"
  else
    absolute="$root/$output"
  fi

  while [[ "$absolute" != "/" && "$absolute" == */ ]]; do
    absolute="${absolute%/}"
  done

  probe="$absolute"
  while [[ ! -e "$probe" ]]; do
    base="$(basename -- "$probe")"
    [[ -n "$base" && "$base" != "/" && "$base" != "." ]] || qa_fail_usage "invalid --out-dir: $output"
    suffix="/$base$suffix"
    probe="$(dirname -- "$probe")"
  done
  [[ -d "$probe" ]] || qa_fail_usage "--out-dir parent is not a directory: $output"

  probe_abs="$(cd -- "$probe" && pwd -P)" || return 1
  printf '%s%s' "$probe_abs" "$suffix"
}

qa_is_allowed_phase_real_surface_dir() {
  local phase_child="$1"

  case "$phase_child" in
    "") return 1 ;;
    parser-fixtures/*/*|parser-fixtures/) return 1 ;;
    parser-fixtures/*) return 0 ;;
    todo-7-parser-repair/fixtures/*/*|todo-7-parser-repair/fixtures/) return 1 ;;
    todo-7-parser-repair/fixtures/*) return 0 ;;
    stale-artifact-repair/fixtures/*/*|stale-artifact-repair/fixtures/) return 1 ;;
    stale-artifact-repair/fixtures/*) return 0 ;;
    todo-7-timeout-parser-repair/*/*|todo-7-timeout-parser-repair/) return 1 ;;
    todo-7-timeout-parser-repair/*-fixture|todo-7-timeout-parser-repair/calendar-event-create-*) return 0 ;;
    */*) return 1 ;;
    real-surface|global-review-real-surface) return 0 ;;
    real-surface-PASS|real-surface-BLOCKED|real-surface-FAIL) return 0 ;;
    real-surface-PASS_TIMEOUT|real-surface-PASS_NONZERO) return 0 ;;
    real-surface-CALENDAR_EVENT_CREATE_*) return 0 ;;
    *) return 1 ;;
  esac
}

qa_has_dedicated_tmp_leaf() {
  local output_abs="$1"
  local leaf

  leaf="$(basename -- "$output_abs")"
  case "$leaf" in
    morrow-lifecycle-real-surface-qa-*|morrow-todo7-*) return 0 ;;
    *) return 1 ;;
  esac
}

qa_is_allowed_tmp_out_dir() {
  local output_abs="$1"
  local tmp_root
  local tmp_root_abs
  local tmp_child
  local seen_roots=" "

  for tmp_root in "${TMPDIR:-/tmp}" /tmp /private/tmp; do
    [[ -d "$tmp_root" ]] || continue
    tmp_root_abs="$(cd -- "$tmp_root" && pwd -P)" || return 1
    case "$seen_roots" in
      *" $tmp_root_abs "*) continue ;;
    esac
    seen_roots="$seen_roots$tmp_root_abs "
    case "$output_abs" in
      "$tmp_root_abs"/*)
        tmp_child="${output_abs#"$tmp_root_abs"/}"
        case "$tmp_child" in
          ""|*/*) continue ;;
        esac
        qa_has_dedicated_tmp_leaf "$output_abs" && return 0
        ;;
    esac
  done

  return 1
}

qa_require_empty_or_marked_tmp_dir() {
  local output_abs="$1"

  [[ -d "$output_abs" ]] || return 0
  [[ -e "$output_abs/.morrow-lifecycle-real-surface-qa-out-dir" ]] && return 0

  if [[ -n "$(find "$output_abs" -mindepth 1 -maxdepth 1 -print -quit)" ]]; then
    qa_fail_usage "--out-dir tmp cleanup requires an empty directory or existing real-surface marker: $output_abs"
  fi
}

qa_clean_out_dir() {
  local output_abs="$1"
  local target

  while IFS= read -r target; do
    case "$target" in
      "$output_abs"/*) rm -rf -- "$target" ;;
      *) qa_fail_usage "refusing to clean path outside --out-dir: $target" ;;
    esac
  done < <(find "$output_abs" -mindepth 1 -maxdepth 1 -print)
}
