#!/usr/bin/env bash
set -euo pipefail

bundle_dir="${1:-}"
evidence_path="${2:-}"
timeout_seconds="${MORROW_SIGNING_QA_TIMEOUT_SECONDS:-30}"
artifacts_ok=0
credentials_ok=0
commands_ok=0
checks=()
tmp_files=()

cleanup() {
  local file index
  for ((index=0; index<${#tmp_files[@]}; index++)); do file="${tmp_files[$index]}"; [ -n "$file" ] && rm -f "$file"; done
}
trap cleanup EXIT

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

sanitize_file() {
  local file="$1" env_name env_value sed_args=()
  if command -v perl >/dev/null 2>&1; then
    perl -0pe 'BEGIN { @s = grep { length } @ENV{qw(APPLE_PASSWORD APPLE_ID APPLE_TEAM_ID APPLE_SIGNING_IDENTITY)}; } for $s (@s) { s/\Q$s\E/[REDACTED_SECRET]/g; } s/(APPLE_(?:PASSWORD|ID|TEAM_ID|SIGNING_IDENTITY)=)[^\s]+/${1}[REDACTED]/g; s/sk-[A-Za-z0-9_-]+/[REDACTED_TOKEN]/g;' "$file"
  else
    sed_args=(-E)
    for env_name in APPLE_PASSWORD APPLE_ID APPLE_TEAM_ID APPLE_SIGNING_IDENTITY; do
      env_value="${!env_name:-}"
      if [ -n "$env_value" ] && [[ "$env_value" != *$'\n'* ]]; then
        env_value="$(printf '%s' "$env_value" | sed -e 's/[.[\*^$()+?{}|\\]/\\&/g' -e 's/]/\\]/g')"
        sed_args+=(-e "s|$env_value|[REDACTED_SECRET]|g")
      fi
    done
    sed_args+=(-e 's/(APPLE_(PASSWORD|ID|TEAM_ID|SIGNING_IDENTITY)=)[^[:space:]]+/\1[REDACTED]/g')
    sed_args+=(-e 's/sk-[A-Za-z0-9_-]+/[REDACTED_TOKEN]/g')
    sed "${sed_args[@]}" "$file"
  fi
}

read_config_signing_identity() {
  local config_path="src-tauri/tauri.conf.json"
  [ -f "$config_path" ] || return 0
  command -v node >/dev/null 2>&1 || return 0
  node -e 'const fs=require("node:fs");try{const c=JSON.parse(fs.readFileSync(process.argv[1],"utf8"));const v=c?.bundle?.macOS?.signingIdentity??c?.bundle?.macos?.signingIdentity??"";if(typeof v==="string")process.stdout.write(v)}catch{}' "$config_path" 2>/dev/null || true
}

discover_artifacts() {
  local root="$1" candidate
  app_paths=()
  dmg_paths=()
  if [ ! -d "$root" ]; then
    record "FAIL" "bundle directory exists" "$root does not exist or is not a directory"
    return
  fi
  record "PASS" "bundle directory exists" "$root"
  while IFS= read -r -d '' candidate; do app_paths+=("$candidate"); done < <(find "$root" -type d -name 'Morrow.app' -prune -print0)
  while IFS= read -r -d '' candidate; do dmg_paths+=("$candidate"); done < <(find "$root" -type f -name '*.dmg' -print0)
  if [ "${#app_paths[@]}" -eq 1 ]; then
    record "PASS" "exactly one Morrow.app bundle is present" "${app_paths[0]}"
  else
    record "FAIL" "exactly one Morrow.app bundle is present" "found ${#app_paths[@]}"
  fi
  if [ "${#dmg_paths[@]}" -eq 1 ]; then
    record "PASS" "exactly one DMG is present" "${dmg_paths[0]}"
  else
    record "FAIL" "exactly one DMG is present" "found ${#dmg_paths[@]}"
  fi
  if [ "${#app_paths[@]}" -eq 1 ] && [ "${#dmg_paths[@]}" -eq 1 ]; then
    artifacts_ok=1
  fi
}

check_credentials() {
  local config_identity signing_source signing_present missing
  config_identity="$(read_config_signing_identity)"
  signing_source="missing"
  signing_present=0
  if [ -n "${APPLE_SIGNING_IDENTITY:-}" ] && [ "${APPLE_SIGNING_IDENTITY:-}" != "-" ]; then
    signing_source="APPLE_SIGNING_IDENTITY present (redacted)"
    signing_present=1
  elif [ -n "$config_identity" ] && [ "$config_identity" != "-" ]; then
    signing_source="bundle.macOS.signingIdentity present (redacted)"
    signing_present=1
  elif [ "${APPLE_SIGNING_IDENTITY:-}" = "-" ] || [ "$config_identity" = "-" ]; then
    signing_source="ad-hoc signing identity is not a release identity"
  fi
  [ "$signing_present" -eq 1 ] && record "PASS" "release signing identity is configured" "$signing_source" || record "BLOCKED" "release signing identity is configured" "$signing_source"
  missing=()
  [ -n "${APPLE_ID:-}" ] || missing+=("APPLE_ID")
  [ -n "${APPLE_PASSWORD:-}" ] || missing+=("APPLE_PASSWORD")
  [ -n "${APPLE_TEAM_ID:-}" ] || missing+=("APPLE_TEAM_ID")
  if [ "${#missing[@]}" -eq 0 ]; then
    record "PASS" "Apple notarization credentials are configured" "APPLE_ID, APPLE_PASSWORD, and APPLE_TEAM_ID are present (redacted)"
  else
    record "BLOCKED" "Apple notarization credentials are configured" "missing ${missing[*]}"
  fi
  if [ "$signing_present" -eq 1 ] && [ "${#missing[@]}" -eq 0 ]; then
    credentials_ok=1
  fi
}

remember_command() {
  command_labels+=("$1")
  command_commands+=("$2")
  command_statuses+=("$3")
  command_output_files+=("$4")
}

run_bounded_command() {
  local label="$1" name="$2" executable output_file timeout_file pid watcher exit_status timed_out transcript
  shift 2
  executable="$1"
  output_file="$(mktemp "${TMPDIR:-/tmp}/morrow-signing-qa-output-XXXXXX")"
  timeout_file="$(mktemp "${TMPDIR:-/tmp}/morrow-signing-qa-timeout-XXXXXX")"
  tmp_files+=("$output_file" "$timeout_file")
  : >"$timeout_file"
  if ! command -v "$executable" >/dev/null 2>&1; then
    printf '%s is not available\n' "$executable" >"$output_file"
    record "FAIL" "$name" "$executable is not available"
    remember_command "$label" "$(quote_args "$@")" "FAIL" "$output_file"
    return
  fi

  "$@" >"$output_file" 2>&1 &
  pid=$!
  ( sleep "$timeout_seconds"; if kill -0 "$pid" 2>/dev/null; then printf 'timeout\n' >"$timeout_file"; kill -TERM "$pid" 2>/dev/null || true; sleep 2; kill -KILL "$pid" 2>/dev/null || true; fi ) &
  watcher=$!
  set +e
  wait "$pid"
  exit_status=$?
  set -e
  if kill -0 "$watcher" 2>/dev/null; then kill "$watcher" 2>/dev/null || true; fi
  wait "$watcher" 2>/dev/null || true

  timed_out=0
  transcript="$output_file"
  if [ -s "$timeout_file" ]; then
    timed_out=1
    exit_status=124
    transcript="${output_file}.timeout"
    { printf 'timed out after %ss\n' "$timeout_seconds"; cat "$output_file"; } >"$transcript"
    tmp_files+=("$transcript")
  fi
  if [ "$exit_status" -eq 0 ] && [ "$timed_out" -eq 0 ]; then
    record "PASS" "$name" "exit 0"
    remember_command "$label" "$(quote_args "$@")" "PASS" "$transcript"
  elif [ "$timed_out" -eq 1 ]; then
    record "FAIL" "$name" "timed out after ${timeout_seconds}s"
    remember_command "$label" "$(quote_args "$@")" "FAIL" "$transcript"
  else
    record "FAIL" "$name" "exit $exit_status"
    remember_command "$label" "$(quote_args "$@")" "FAIL" "$transcript"
  fi
}

write_evidence() {
  local result="$1" env_name check status name details index output
  mkdir -p "$(dirname "$evidence_path")"
  {
    printf '# Task 3 Beta Release Distribution Evidence\n\n'
    printf '## Invocation\n\n`%s`\n\n' "$(quote_args scripts/release-signing-qa.sh "$bundle_dir" "$evidence_path")"
    printf '## Inputs\n\n- Bundle directory: `%s`\n- Evidence path: `%s`\n- Command timeout: `%ss`\n\n' "$bundle_dir" "$evidence_path" "$timeout_seconds"
    printf '## Sanitized Credential State\n\n'
    for env_name in APPLE_SIGNING_IDENTITY APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID; do
      if [ -n "${!env_name:-}" ]; then
        printf -- '- %s: present (redacted)\n' "$env_name"
      else
        printf -- '- %s: missing\n' "$env_name"
      fi
    done
    printf '\n## Discovered Artifacts\n\n- Morrow.app bundles: %s\n' "${#app_paths[@]}"
    for ((index=0; index<${#app_paths[@]}; index++)); do printf '  - `%s`\n' "${app_paths[$index]}"; done
    printf -- '- DMG files: %s\n' "${#dmg_paths[@]}"
    for ((index=0; index<${#dmg_paths[@]}; index++)); do printf '  - `%s`\n' "${dmg_paths[$index]}"; done
    printf '\n## Assertions\n\n'
    for check in "${checks[@]}"; do
      IFS=$'\t' read -r status name details <<<"$check"
      printf -- '- %s %s: %s\n' "$status" "$name" "$details"
    done
    printf '\n## Command Transcripts\n\n'
    if [ "${#command_labels[@]}" -eq 0 ]; then
      printf 'No signing commands were run because artifact discovery or credential checks blocked release-gate execution.\n'
    else
      for index in "${!command_labels[@]}"; do
        printf '### %s\n\n- Command: `%s`\n- Result: %s\n\n```text\n' "${command_labels[$index]}" "${command_commands[$index]}" "${command_statuses[$index]}"
        output="$(sanitize_file "${command_output_files[$index]}")"
        printf '%s\n```\n\n' "$output"
      done
    fi
    printf 'RESULT: %s\n' "$result"
  } >"$evidence_path"
}

if [ -z "$bundle_dir" ] || [ -z "$evidence_path" ]; then
  printf 'Usage: scripts/release-signing-qa.sh <bundle-dir> <evidence.md>\n' >&2
  exit 2
fi
if ! [[ "$timeout_seconds" =~ ^[0-9]+$ ]] || [ "$timeout_seconds" -le 0 ]; then
  printf 'release-signing-qa: MORROW_SIGNING_QA_TIMEOUT_SECONDS must be a positive integer\n' >&2
  exit 2
fi

app_paths=()
dmg_paths=()
command_labels=()
command_commands=()
command_statuses=()
command_output_files=()
discover_artifacts "$bundle_dir"
check_credentials

if [ "$artifacts_ok" -eq 1 ] && [ "$credentials_ok" -eq 1 ]; then
  APP="${app_paths[0]}"
  DMG="${dmg_paths[0]}"
  run_bounded_command "codesign verify app" "codesign verifies app bundle" codesign --verify --deep --strict --verbose=2 "$APP"
  run_bounded_command "codesign details app" "codesign details are readable" codesign -dv "$APP"
  run_bounded_command "spctl assess app" "spctl accepts app for execution" spctl --assess --type execute --verbose=4 "$APP"
  run_bounded_command "stapler validate app" "stapler validates notarization ticket on app" xcrun stapler validate "$APP"
  run_bounded_command "spctl assess dmg" "spctl accepts DMG primary signature" spctl --assess --type open --context context:primary-signature --verbose=4 "$DMG"
  if [ "${#command_output_files[@]}" -gt 0 ] && grep -E 'Signature=adhoc|Authority=Ad Hoc' "${command_output_files[@]}" >/dev/null 2>&1; then
    record "FAIL" "app signature is not ad-hoc" "codesign details reported an ad-hoc signature"
  else
    record "PASS" "app signature is not ad-hoc" "codesign details did not report ad-hoc signing"
  fi
else
  record "BLOCKED" "release signing command execution" "artifact discovery and credential checks must pass before signed/notarized release verification"
fi

if [ "${#checks[@]}" -gt 0 ] && ! printf '%s\n' "${checks[@]}" | grep -qE '^(FAIL|BLOCKED)'$'\t'; then
  commands_ok=1
fi
if [ "$artifacts_ok" -eq 1 ] && [ "$credentials_ok" -eq 1 ] && [ "$commands_ok" -eq 1 ]; then
  write_evidence "PASS"
  exit 0
fi
if printf '%s\n' "${checks[@]}" | grep -q '^BLOCKED'$'\t'; then
  write_evidence "BLOCKED"
else
  write_evidence "FAIL"
fi
exit 1
