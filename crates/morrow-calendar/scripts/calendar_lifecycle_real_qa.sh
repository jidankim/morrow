#!/usr/bin/env zsh
set -u

if [[ "$(uname -s)" != "Darwin" ]]; then
  printf 'BLOCKED: real Calendar lifecycle QA requires macOS Calendar/EventKit\n' >&2
  exit 64
fi

script_dir="${0:A:h}"
objc_source="${script_dir}/calendar_lifecycle_real_qa.m"
command_display="crates/morrow-calendar/scripts/calendar_lifecycle_real_qa.sh"
binary_path="${TMPDIR:-/tmp}/morrow-calendar-lifecycle-real-qa-$$"

clang_output=$(/usr/bin/clang -fobjc-arc -Wall -Wextra -Werror -framework Foundation -framework EventKit "$objc_source" -o "$binary_path" 2>&1)
clang_status=$?
if (( clang_status != 0 )); then
  printf '%s\n' "$clang_output"
  exit "$clang_status"
fi

qa_output=$("$binary_path" "$command_display" 2>&1)
qa_status=$?
rm -f "$binary_path"
printf '%s\n' "$qa_output"

if (( qa_status != 0 )); then
  case "$qa_output" in
    *"BLOCKED:"*)
      ;;
    *"not authorized"*|*"Not authorized"*|*"authorization denied"*|*"access denied"*|*"permission"*)
      printf 'BLOCKED: macOS Calendar permission is required.\n'
      printf 'required_user_action=Grant this terminal/Codex process Calendar access, then rerun: %s\n' "$command_display"
      ;;
  esac
  exit "$qa_status"
fi
