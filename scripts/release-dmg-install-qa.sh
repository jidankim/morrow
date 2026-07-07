#!/usr/bin/env bash
set -euo pipefail

bundle_dir="${1:-}"
evidence_path="${2:-}"
checks=()
dmg_paths=()
mount_point=""
attach_output=""

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
  local result check status name details
  result="$(result_status)"
  mkdir -p "$(dirname "$evidence_path")"
  {
    printf '# Task 7 Beta DMG Install QA\n\n'
    printf '## Invocation\n\n`%s`\n\n' "$(quote_args scripts/release-dmg-install-qa.sh "$bundle_dir" "$evidence_path")"
    printf '## Inputs\n\n- Bundle directory: `%s`\n- Evidence path: `%s`\n\n' "${bundle_dir:-"(missing)"}" "${evidence_path:-"(missing)"}"
    printf '## Discovered DMGs\n\n- Count: %s\n' "${#dmg_paths[@]}"
    local dmg
    for dmg in "${dmg_paths[@]}"; do
      printf -- '- `%s`\n' "$dmg"
    done
    printf '\n## Mount\n\n- Mount point: `%s`\n\n' "${mount_point:-"(none)"}"
    if [ -n "$attach_output" ]; then
      printf 'Attach output:\n\n```text\n%s\n```\n\n' "$attach_output"
    fi
    printf '## Assertions\n\n'
    for check in "${checks[@]}"; do
      IFS=$'\t' read -r status name details <<<"$check"
      printf -- '- %s %s: %s\n' "$status" "$name" "$details"
    done
    printf '\n## Cleanup Receipt\n\n'
    printf -- '- hdiutil info checked after QA.\n'
    if [ -n "$mount_point" ]; then
      if hdiutil info | grep -F "$mount_point" >/dev/null 2>&1; then
        printf -- '- Leftover mount still present: `%s`\n' "$mount_point"
      else
        printf -- '- No leftover mount for `%s`.\n' "$mount_point"
      fi
    else
      printf -- '- No mount point was created.\n'
    fi
    printf '\nRESULT: %s\n' "$result"
  } >"$evidence_path"
}

detach_mount() {
  if [ -n "$mount_point" ] && hdiutil info | grep -F "$mount_point" >/dev/null 2>&1; then
    if hdiutil detach "$mount_point" >/dev/null 2>&1; then
      record "PASS" "DMG volume detached" "$mount_point"
    else
      record "FAIL" "DMG volume detached" "$mount_point did not detach"
    fi
  fi
}

cleanup() {
  detach_mount || true
  if [ -n "${evidence_path:-}" ]; then
    write_evidence || true
  fi
}
trap cleanup EXIT

if [ -z "$bundle_dir" ] || [ -z "$evidence_path" ]; then
  printf 'Usage: scripts/release-dmg-install-qa.sh <bundle-dir> <evidence.md>\n' >&2
  exit 2
fi

if [ ! -d "$bundle_dir" ]; then
  record "BLOCKED" "bundle directory exists" "$bundle_dir does not exist or is not a directory"
else
  record "PASS" "bundle directory exists" "$bundle_dir"
  while IFS= read -r -d '' dmg; do
    dmg_paths+=("$dmg")
  done < <(find "$bundle_dir" -type f -name '*.dmg' -print0)
  if [ "${#dmg_paths[@]}" -eq 1 ]; then
    record "PASS" "exactly one DMG is present" "${dmg_paths[0]}"
  else
    record "BLOCKED" "exactly one DMG is present" "found ${#dmg_paths[@]}"
  fi
fi

if ! command -v hdiutil >/dev/null 2>&1; then
  record "BLOCKED" "hdiutil is available" "hdiutil is required for DMG mount QA"
elif [ "${#dmg_paths[@]}" -eq 1 ]; then
  attach_output="$(hdiutil attach -nobrowse -readonly "${dmg_paths[0]}" 2>&1 || true)"
  mount_point="$(printf '%s\n' "$attach_output" | awk '/\/Volumes\// { print substr($0, index($0, "/Volumes/")); exit }')"
  if [ -n "$mount_point" ] && [ -d "$mount_point" ]; then
    record "PASS" "DMG mounts read-only" "$mount_point"
    app_path="$(find "$mount_point" -maxdepth 2 -type d -name 'Morrow.app' -print -quit)"
    if [ -n "$app_path" ]; then
      record "PASS" "mounted volume contains Morrow.app" "$app_path"
      plist_path="$app_path/Contents/Info.plist"
      executable_name=""
      if [ -f "$plist_path" ] && command -v plutil >/dev/null 2>&1; then
        executable_name="$(plutil -extract CFBundleExecutable raw -o - "$plist_path" 2>/dev/null || true)"
      fi
      [ -n "$executable_name" ] || executable_name="Morrow"
      if [ -f "$app_path/Contents/MacOS/$executable_name" ]; then
        record "PASS" "mounted app has Contents/MacOS executable" "$app_path/Contents/MacOS/$executable_name"
      else
        record "FAIL" "mounted app has Contents/MacOS executable" "$app_path/Contents/MacOS/$executable_name is missing"
      fi
    else
      record "FAIL" "mounted volume contains Morrow.app" "$mount_point"
      record "FAIL" "mounted app has Contents/MacOS executable" "no Morrow.app in mounted DMG"
    fi
    if [ -L "$mount_point/Applications" ] || [ -e "$mount_point/Applications" ]; then
      record "PASS" "Applications link is optional and accepted" "$mount_point/Applications present"
    else
      record "PASS" "Applications link is optional and accepted" "not present"
    fi
  else
    record "FAIL" "DMG mounts read-only" "$attach_output"
  fi
fi

if [ "$(result_status)" = "PASS" ]; then
  exit 0
fi
exit 1
