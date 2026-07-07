#!/usr/bin/env bash
set -euo pipefail

bundle_dir="${1:-}"
evidence_path="${2:-}"
checks=()
app_paths=()
launched_pids=()
temp_root=""
screenshot_path=""

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

result_status() {
  if printf '%s\n' "${checks[@]}" | grep -q '^BLOCKED'$'\t'; then
    printf 'BLOCKED'
  elif printf '%s\n' "${checks[@]}" | grep -q '^FAIL'$'\t'; then
    printf 'FAIL'
  else
    printf 'PASS'
  fi
}

write_evidence() {
  local result check status name details pid pgrep_output
  result="$(result_status)"
  mkdir -p "$(dirname "$evidence_path")"
  pgrep_output="$(pgrep -fl 'Morrow|morrow' 2>/dev/null || true)"
  {
    printf '# Task 8 Beta Packaged App Launch QA\n\n'
    printf '## Invocation\n\n`%s`\n\n' "$(quote_args scripts/release-packaged-app-launch-qa.sh "$bundle_dir" "$evidence_path")"
    printf '## Inputs\n\n- Bundle directory: `%s`\n- Evidence path: `%s`\n- Screenshot path: `%s`\n\n' "${bundle_dir:-"(missing)"}" "${evidence_path:-"(missing)"}" "${screenshot_path:-"(not created)"}"
    printf '## Discovered Apps\n\n- Count: %s\n' "${#app_paths[@]}"
    local app
    for app in "${app_paths[@]}"; do
      printf -- '- `%s`\n' "$app"
    done
    printf '\n## Launched Processes\n\n'
    if [ "${#launched_pids[@]}" -eq 0 ]; then
      printf -- '- none\n'
    else
      for pid in "${launched_pids[@]}"; do
        printf -- '- `%s`\n' "$pid"
      done
    fi
    printf '\n## Assertions\n\n'
    for check in "${checks[@]}"; do
      IFS=$'\t' read -r status name details <<<"$check"
      printf -- '- %s %s: %s\n' "$status" "$name" "$details"
    done
    printf '\n## Cleanup Receipt\n\n'
    if [ -n "$pgrep_output" ]; then
      printf 'Post-QA process scan:\n\n```text\n%s\n```\n\n' "$pgrep_output"
    else
      printf 'Post-QA process scan: no `Morrow|morrow` process found.\n\n'
    fi
    if [ -n "$temp_root" ] && [ -e "$temp_root" ]; then
      printf -- '- Temp app copy still exists: `%s`\n' "$temp_root"
    else
      printf -- '- Temp app copy removed.\n'
    fi
    printf '\nRESULT: %s\n' "$result"
  } >"$evidence_path"
}

cleanup() {
  if [ "${#launched_pids[@]}" -gt 0 ]; then
    osascript -e 'tell application "Morrow" to quit' >/dev/null 2>&1 || true
    sleep 2
    local pid
    for pid in "${launched_pids[@]}"; do
      if kill -0 "$pid" >/dev/null 2>&1; then
        kill "$pid" >/dev/null 2>&1 || true
      fi
    done
  fi
  if [ -n "$temp_root" ]; then
    rm -rf "$temp_root"
  fi
  if [ -n "${evidence_path:-}" ]; then
    write_evidence || true
  fi
}
trap cleanup EXIT

if [ -z "$bundle_dir" ] || [ -z "$evidence_path" ]; then
  printf 'Usage: scripts/release-packaged-app-launch-qa.sh <bundle-dir> <evidence.md>\n' >&2
  exit 2
fi

screenshot_path="${evidence_path%.*}.png"

if [ ! -d "$bundle_dir" ]; then
  record "BLOCKED" "bundle directory exists" "$bundle_dir does not exist or is not a directory"
else
  record "PASS" "bundle directory exists" "$bundle_dir"
  while IFS= read -r -d '' app; do
    app_paths+=("$app")
  done < <(find "$bundle_dir" -type d -name 'Morrow.app' -prune -print0)
  if [ "${#app_paths[@]}" -eq 1 ]; then
    record "PASS" "exactly one Morrow.app bundle is present" "${app_paths[0]}"
  else
    record "BLOCKED" "exactly one Morrow.app bundle is present" "found ${#app_paths[@]}"
  fi
fi

if [ "${#app_paths[@]}" -eq 1 ]; then
  if pgrep -x Morrow >/dev/null 2>&1; then
    record "BLOCKED" "no pre-existing Morrow process is running" "refusing to quit an unrelated app process"
  elif ! command -v open >/dev/null 2>&1 || ! command -v screencapture >/dev/null 2>&1; then
    record "BLOCKED" "desktop automation tools are available" "open and screencapture are required"
  else
    temp_root="$(mktemp -d /tmp/morrow-beta-qa-launch-XXXXXX)"
    if command -v ditto >/dev/null 2>&1; then
      ditto "${app_paths[0]}" "$temp_root/Morrow.app"
    else
      cp -R "${app_paths[0]}" "$temp_root/Morrow.app"
    fi
    record "PASS" "packaged app copied to temp location" "$temp_root/Morrow.app"
    open -n "$temp_root/Morrow.app"
    for _ in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30; do
      launched_pids=()
      while IFS= read -r pid; do
        [ -n "$pid" ] && launched_pids+=("$pid")
      done < <(pgrep -x Morrow || true)
      [ "${#launched_pids[@]}" -gt 0 ] && break
      sleep 1
    done
    if [ "${#launched_pids[@]}" -gt 0 ]; then
      record "PASS" "packaged app process starts" "${launched_pids[*]}"
    else
      record "FAIL" "packaged app process starts" "no Morrow process appeared"
    fi
    if [ "${#launched_pids[@]}" -gt 0 ] && screencapture -x "$screenshot_path" >/dev/null 2>&1 && [ -s "$screenshot_path" ]; then
      record "PASS" "packaged app launch screenshot captured" "$screenshot_path"
    else
      record "FAIL" "packaged app launch screenshot captured" "$screenshot_path missing or empty"
    fi
  fi
fi

if [ "$(result_status)" = "PASS" ]; then
  exit 0
fi
exit 1
