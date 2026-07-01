#!/usr/bin/env bash
# noqa: SIZE_OK - single-purpose diagnostic quarantine QA keeps downloaded-app simulation, Gatekeeper probes, and reporting together; split if it grows further.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

timeout_seconds="${MORROW_DIAGNOSTIC_QA_TIMEOUT_SECONDS:-45}"
original_args=("$@")
checks=()
command_labels=()
command_commands=()
command_statuses=()
command_outputs=()
tmp_files=()
temp_dir=""
cleanup_receipts=()

usage() {
  printf 'Usage: %s <diagnostic-dir> <evidence.md>\n' "$0" >&2
}

quote_args() {
  local first=1 arg
  for arg in "$@"; do
    [ "$first" -eq 0 ] && printf ' '
    first=0
    printf '%q' "$arg"
  done
}

record() {
  checks+=("$1"$'\t'"$2"$'\t'"$3")
  printf '%s %s: %s\n' "$1" "$2" "$3"
}

sanitize_text() {
  if command -v perl >/dev/null 2>&1; then
    perl -0pe '
      s/(APPLE_(?:PASSWORD|ID|TEAM_ID|SIGNING_IDENTITY)=)[^\s]+/${1}[REDACTED]/g;
      s/\bsk-[A-Za-z0-9][A-Za-z0-9_-]{20,}/[REDACTED_TOKEN]/g;
      s/codex_access_token[^\s]*/codex_access_token=[REDACTED]/gi;
      s/auth\.json/[REDACTED_AUTH_FILE]/gi;
    '
  else
    sed -E \
      -e 's/(APPLE_(PASSWORD|ID|TEAM_ID|SIGNING_IDENTITY)=)[^[:space:]]+/\1[REDACTED]/g' \
      -e 's/\bsk-[A-Za-z0-9][A-Za-z0-9_-]{20,}/[REDACTED_TOKEN]/g' \
      -e 's/codex_access_token[^[:space:]]*/codex_access_token=[REDACTED]/Ig' \
      -e 's/auth\.json/[REDACTED_AUTH_FILE]/Ig'
  fi
}

sanitize_file() {
  sanitize_text <"$1"
}

remember_command() {
  command_labels+=("$1")
  command_commands+=("$2")
  command_statuses+=("$3")
  command_outputs+=("$4")
}

run_bounded_command() {
  local label="$1" output_file timeout_file pid watcher exit_status timed_out command_text executable
  shift
  executable="$1"
  shift
  output_file="$(mktemp "${TMPDIR:-/tmp}/morrow-diagnostic-quarantine-command-XXXXXX")"
  timeout_file="$(mktemp "${TMPDIR:-/tmp}/morrow-diagnostic-quarantine-timeout-XXXXXX")"
  tmp_files+=("$output_file" "$timeout_file")
  : >"$timeout_file"
  command_text="$(quote_args "$@")"

  if ! command -v "$executable" >/dev/null 2>&1; then
    printf '%s is not available\n' "$executable" >"$output_file"
    remember_command "$label" "$command_text" "missing command" "$output_file"
    return 127
  fi

  "$@" >"$output_file" 2>&1 &
  pid=$!
  (
    sleep "$timeout_seconds"
    if kill -0 "$pid" 2>/dev/null; then
      printf 'timeout\n' >"$timeout_file"
      kill -TERM "$pid" 2>/dev/null || true
      sleep 2
      kill -KILL "$pid" 2>/dev/null || true
    fi
  ) &
  watcher=$!

  set +e
  wait "$pid"
  exit_status=$?
  if kill -0 "$watcher" 2>/dev/null; then
    kill "$watcher" 2>/dev/null || true
  fi
  wait "$watcher" 2>/dev/null || true

  timed_out=0
  if [ -s "$timeout_file" ]; then
    timed_out=1
    exit_status=124
  fi

  if [ "$timed_out" -eq 1 ]; then
    remember_command "$label" "$command_text" "timeout ${timeout_seconds}s" "$output_file"
  else
    remember_command "$label" "$command_text" "exit $exit_status" "$output_file"
  fi
  return "$exit_status"
}

cleanup_temp() {
  if [ -n "$temp_dir" ] && [[ "$temp_dir" == /tmp/morrow-diagnostic-qa-* ]] && [ -e "$temp_dir" ]; then
    rm -rf "$temp_dir"
    cleanup_receipts+=("removed temp quarantine copy $temp_dir")
  fi
}

find_primary_zip() {
  local diagnostic_dir="$1" list_file="$2" count
  : >"$list_file"
  if [ -d "$diagnostic_dir" ]; then
    find "$diagnostic_dir" -maxdepth 1 -type f -name '*.zip' -print >"$list_file"
  fi
  count="$(wc -l <"$list_file" | tr -d ' ')"
  printf '%s' "$count"
}

find_unpacked_app() {
  local root="$1" list_file="$2" count
  : >"$list_file"
  if [ -d "$root" ]; then
    find "$root" -type d -name 'Morrow.app' -prune -print >"$list_file"
  fi
  count="$(wc -l <"$list_file" | tr -d ' ')"
  printf '%s' "$count"
}

write_evidence() {
  local diagnostic_dir="$1" evidence_path="$2" result="$3" primary_zip="${4:-}" app_path="${5:-}" cleanup_scan output index
  mkdir -p "$(dirname "$evidence_path")"
  cleanup_scan="$(find /tmp -maxdepth 1 -name 'morrow-diagnostic-qa-*' -print 2>/dev/null || true)"
  {
    printf '# Todo 6 Diagnostic Quarantine QA Evidence\n\n'
    printf '## Scenario\n\n'
    printf -- '- Invocation: `%s`\n' "$(quote_args "$0" "${original_args[@]}")"
    printf -- '- Diagnostic directory: `%s`\n' "$diagnostic_dir"
    printf -- '- Evidence path: `%s`\n' "$evidence_path"
    printf -- '- Primary zip: `%s`\n' "${primary_zip:-missing}"
    printf -- '- Quarantined temp app copy: `%s`\n\n' "${app_path:-missing}"
    printf '## Assertions\n\n'
    for check in "${checks[@]}"; do
      local status name details
      IFS=$'\t' read -r status name details <<<"$check"
      printf -- '- %s %s: %s\n' "$status" "$name" "$details"
    done
    printf '\n## Command Transcripts\n\n'
    if [ "${#command_labels[@]}" -eq 0 ]; then
      printf 'No external command transcripts were captured.\n\n'
    else
      for index in "${!command_labels[@]}"; do
        printf '### %s\n\n' "${command_labels[$index]}"
        printf -- '- Command: `%s`\n' "${command_commands[$index]}"
        printf -- '- Result: %s\n\n' "${command_statuses[$index]}"
        printf '```text\n'
        output="$(sanitize_file "${command_outputs[$index]}")"
        printf '%s\n' "$output"
        printf '```\n\n'
      done
    fi
    printf '## Cleanup\n\n'
    if [ "${#cleanup_receipts[@]}" -eq 0 ]; then
      printf -- '- No temp quarantine directory was created.\n'
    else
      printf -- '- %s\n' "${cleanup_receipts[@]}"
    fi
    if [ -z "$cleanup_scan" ]; then
      printf -- '- PASS cleanup: no `/tmp/morrow-diagnostic-qa-*` directories remain.\n'
    else
      printf -- '- FAIL cleanup: leftover temp directories remain:\n```text\n%s\n```\n' "$cleanup_scan"
    fi
    printf '\nRESULT: %s\n' "$result"
  } | sanitize_text >"$evidence_path"
}

main() {
  local diagnostic_dir="${1:-}" evidence_path="${2:-}" zip_list app_list zip_count primary_zip unzip_status app_count app_path quarantine_value
  local xattr_status attr_status spctl_before_status spctl_status spctl_after_status result executable_path

  if [ -z "$diagnostic_dir" ] || [ -z "$evidence_path" ]; then
    usage
    exit 2
  fi
  if ! [[ "$timeout_seconds" =~ ^[0-9]+$ ]] || [ "$timeout_seconds" -le 0 ]; then
    printf 'diagnostic-quarantine-qa: MORROW_DIAGNOSTIC_QA_TIMEOUT_SECONDS must be a positive integer\n' >&2
    exit 2
  fi

  temp_dir="$(mktemp -d /tmp/morrow-diagnostic-qa-XXXXXX)"
  zip_list="$(mktemp "${TMPDIR:-/tmp}/morrow-diagnostic-quarantine-zips-XXXXXX")"
  app_list="$(mktemp "${TMPDIR:-/tmp}/morrow-diagnostic-quarantine-apps-XXXXXX")"
  tmp_files+=("$zip_list" "$app_list")

  if [ -d "$diagnostic_dir" ]; then
    record "PASS" "diagnostic payload directory exists" "$diagnostic_dir"
  else
    record "FAIL" "diagnostic payload directory exists" "$diagnostic_dir"
  fi

  zip_count="$(find_primary_zip "$diagnostic_dir" "$zip_list")"
  if [ "$zip_count" -eq 1 ]; then
    primary_zip="$(sed -n '1p' "$zip_list")"
    record "PASS" "diagnostic payload has exactly one primary zip" "$(basename "$primary_zip")"
  else
    primary_zip=""
    record "FAIL" "diagnostic payload has exactly one primary zip" "found $zip_count"
  fi

  if [ -n "$primary_zip" ]; then
    set +e
    run_bounded_command "unpack primary diagnostic zip" unzip unzip -q "$primary_zip" -d "$temp_dir/unpacked"
    unzip_status=$?
    set -e
    [ "$unzip_status" -eq 0 ] && record "PASS" "primary zip unpacks into temp copy" "$temp_dir/unpacked" || record "FAIL" "primary zip unpacks into temp copy" "unzip exit $unzip_status"
  else
    record "FAIL" "primary zip unpacks into temp copy" "no primary zip"
  fi

  app_count="$(find_unpacked_app "$temp_dir/unpacked" "$app_list")"
  if [ "$app_count" -eq 1 ]; then
    app_path="$(sed -n '1p' "$app_list")"
    record "PASS" "Morrow.app comes from packaged diagnostic zip temp copy" "$app_path"
  else
    app_path=""
    record "FAIL" "Morrow.app comes from packaged diagnostic zip temp copy" "found $app_count"
  fi

  if [ -n "$app_path" ]; then
    quarantine_value="0081;$(date +%s);MorrowDiagnosticQA;$(uuidgen 2>/dev/null || printf diagnostic)"
    set +e
    run_bounded_command "apply quarantine xattr to temp app copy" xattr xattr -w com.apple.quarantine "$quarantine_value" "$app_path"
    xattr_status=$?
    set -e
    [ "$xattr_status" -eq 0 ] && record "PASS" "quarantine attribute applied only to temp app copy" "$app_path" || record "FAIL" "quarantine attribute applied only to temp app copy" "xattr exit $xattr_status"

    set +e
    run_bounded_command "read quarantine xattr from temp app copy" xattr xattr -p com.apple.quarantine "$app_path"
    attr_status=$?
    set -e
    if [ "$attr_status" -eq 0 ] && grep -q 'MorrowDiagnosticQA' "${command_outputs[$((${#command_outputs[@]} - 1))]}"; then
      record "PASS" "quarantine attribute readback proves downloaded-artifact simulation" "com.apple.quarantine marker present"
    else
      record "FAIL" "quarantine attribute readback proves downloaded-artifact simulation" "attribute missing or unreadable"
    fi

    executable_path="$(find "$app_path/Contents/MacOS" -type f -perm -111 -print -quit 2>/dev/null || true)"
    if [ -n "$executable_path" ]; then
      record "PASS" "temp app executable exists for Gatekeeper assessment" "$executable_path"
    else
      record "FAIL" "temp app executable exists for Gatekeeper assessment" "missing executable"
    fi
  else
    record "FAIL" "quarantine attribute applied only to temp app copy" "no Morrow.app"
    record "FAIL" "quarantine attribute readback proves downloaded-artifact simulation" "no Morrow.app"
    record "FAIL" "temp app executable exists for Gatekeeper assessment" "no Morrow.app"
  fi

  set +e
  run_bounded_command "spctl status before quarantine probe" spctl spctl --status
  spctl_before_status=$?
  set -e
  if [ "$spctl_before_status" -eq 0 ] && grep -qi 'assessments enabled' "${command_outputs[$((${#command_outputs[@]} - 1))]}"; then
    record "PASS" "global Gatekeeper assessments remain enabled before probe" "spctl --status reports enabled"
  else
    record "FAIL" "global Gatekeeper assessments remain enabled before probe" "spctl --status did not report enabled"
  fi

  if [ -n "$app_path" ]; then
    set +e
    run_bounded_command "spctl assess quarantined temp app" spctl spctl --assess --type execute --verbose=4 "$app_path"
    spctl_status=$?
    set -e
    if [ "$spctl_status" -ne 0 ]; then
      record "PASS" "quarantined ad-hoc diagnostic app is blocked or requires approval as expected" "spctl exit $spctl_status"
    else
      record "FAIL" "quarantined ad-hoc diagnostic app is blocked or requires approval as expected" "spctl unexpectedly accepted"
    fi
  else
    record "FAIL" "quarantined ad-hoc diagnostic app is blocked or requires approval as expected" "no Morrow.app"
  fi

  set +e
  run_bounded_command "spctl status after quarantine probe" spctl spctl --status
  spctl_after_status=$?
  set -e
  if [ "$spctl_after_status" -eq 0 ] && grep -qi 'assessments enabled' "${command_outputs[$((${#command_outputs[@]} - 1))]}"; then
    record "PASS" "no global Gatekeeper security changes were made" "spctl --status still reports enabled; script never invokes spctl --master-disable"
  else
    record "FAIL" "no global Gatekeeper security changes were made" "spctl --status did not report enabled after probe"
  fi

  cleanup_temp
  if [ -n "$temp_dir" ] && [ ! -e "$temp_dir" ]; then
    record "PASS" "temporary quarantine copy cleanup" "$temp_dir removed"
  else
    record "FAIL" "temporary quarantine copy cleanup" "${temp_dir:-missing} still exists"
  fi

  if printf '%s\n' "${checks[@]}" | grep -q '^FAIL'$'\t'; then
    result="FAIL"
  else
    result="PASS"
  fi
  write_evidence "$diagnostic_dir" "$evidence_path" "$result" "$primary_zip" "$app_path"
  rm -f "${tmp_files[@]}" 2>/dev/null || true
  [ "$result" = "PASS" ]
}

main "$@"
