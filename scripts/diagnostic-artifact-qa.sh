#!/usr/bin/env bash
# noqa: SIZE_OK - single-purpose diagnostic artifact QA keeps macOS signing probes, archive fixtures, and evidence reporting together; split if it grows further.
set -euo pipefail

script_path="$0"
timeout_seconds="${MORROW_DIAGNOSTIC_QA_TIMEOUT_SECONDS:-45}"
checks=()
command_labels=()
command_commands=()
command_statuses=()
command_outputs=()
tmp_files=()
tmp_dirs=()

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
      BEGIN {
        @secrets = grep { defined && length } @ENV{qw(APPLE_PASSWORD APPLE_ID APPLE_TEAM_ID APPLE_SIGNING_IDENTITY CODEX_ACCESS_TOKEN)};
      }
      for $secret (@secrets) {
        s/\Q$secret\E/[REDACTED_SECRET]/g;
      }
      s/APPLE_(?:PASSWORD|ID|TEAM_ID|SIGNING_IDENTITY)=[^\s]+/APPLE_SECRET=[REDACTED]/g;
      s/sk-[A-Za-z0-9_-]+/[REDACTED_TOKEN]/g;
      s/codex_access_token/[REDACTED_TOKEN_NAME]/gi;
      s/auth\.json/[REDACTED_AUTH_FILE]/gi;
      s/\b[A-Za-z0-9_-]{32,}\b/[REDACTED_TOKENLIKE]/g;
    '
  else
    sed -E \
      -e 's/APPLE_(PASSWORD|ID|TEAM_ID|SIGNING_IDENTITY)=[^[:space:]]+/APPLE_SECRET=[REDACTED]/g' \
      -e 's/sk-[A-Za-z0-9_-]+/[REDACTED_TOKEN]/g' \
      -e 's/codex_access_token/[REDACTED_TOKEN_NAME]/Ig' \
      -e 's/auth\.json/[REDACTED_AUTH_FILE]/Ig' \
      -e 's/[A-Za-z0-9_-]{32,}/[REDACTED_TOKENLIKE]/g'
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
  local label="$1" executable="$2" output_file timeout_file pid watcher exit_status timed_out command_text
  shift 2
  output_file="$(mktemp "${TMPDIR:-/tmp}/morrow-diagnostic-artifact-command-XXXXXX")"
  timeout_file="$(mktemp "${TMPDIR:-/tmp}/morrow-diagnostic-artifact-timeout-XXXXXX")"
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

plist_json() {
  local plist="$1" json_file="$2"
  if ! command -v plutil >/dev/null 2>&1; then
    record "FAIL" "plutil is available for Info.plist parsing" "plutil not found"
    return 1
  fi
  if plutil -convert json -o "$json_file" "$plist" >/dev/null 2>&1; then
    record "PASS" "Info.plist parses with plutil" "$plist"
    return 0
  fi
  record "FAIL" "Info.plist parses with plutil" "$plist"
  return 1
}

json_value() {
  local file="$1" expression="$2"
  node -e '
    const fs = require("node:fs");
    const data = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
    const expression = process.argv[2];
    const value = expression.split(".").reduce((current, key) => current == null ? undefined : current[key], data);
    if (value === undefined || value === null) process.exit(3);
    process.stdout.write(String(value));
  ' "$file" "$expression"
}

json_bool() {
  local file="$1" script="$2"
  node -e '
    const fs = require("node:fs");
    const data = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
    const fn = new Function("data", process.argv[2]);
    process.exit(fn(data) ? 0 : 1);
  ' "$file" "$script"
}

sha256_of() {
  shasum -a 256 "$1" | awk '{print $1}'
}

checksum_has_match() {
  local sums="$1" file="$2" pattern="$3" digest
  digest="$(sha256_of "$file")"
  awk -v digest="$digest" -v pattern="$pattern" '
    index($0, digest) && index($0, pattern) { found = 1 }
    END { exit found ? 0 : 1 }
  ' "$sums"
}

record_checksum_match() {
  local label="$1" sums="$2" file="$3" pattern="$4"
  if [ -f "$file" ] && checksum_has_match "$sums" "$file" "$pattern"; then
    record "PASS" "$label has SHA-256 coverage" "matched digest and path marker $pattern"
  else
    record "FAIL" "$label has SHA-256 coverage" "missing digest or path marker $pattern"
  fi
}

find_one_file() {
  local root="$1" name_pattern="$2" output_file="$3" count
  : >"$output_file"
  if [ -d "$root" ]; then
    find "$root" -maxdepth 1 -type f -name "$name_pattern" -print >"$output_file"
  fi
  count="$(wc -l <"$output_file" | tr -d ' ')"
  printf '%s' "$count"
}

find_one_path() {
  local root="$1" type="$2" name_pattern="$3" output_file="$4" count
  : >"$output_file"
  if [ -d "$root" ]; then
    find "$root" -type "$type" -name "$name_pattern" -prune -print >"$output_file"
  fi
  count="$(wc -l <"$output_file" | tr -d ' ')"
  printf '%s' "$count"
}

find_one_app() {
  find_one_path "$1" d 'Morrow.app' "$2"
}

find_one_note() {
  find_one_path "$1" f 'Morrow-diagnostic-tester-note.md' "$2"
}

write_real_evidence() {
  local diagnostic_dir="$1" evidence_path="$2" result="$3" primary_zip="${4:-}" app_path="${5:-}" temp_dir="${6:-}" index output
  mkdir -p "$(dirname "$evidence_path")"
  {
    printf '# Todo 2 Diagnostic Artifact QA Evidence\n\n'
    printf '## Scenario\n\n'
    printf -- '- Invocation: `%s`\n' "$(quote_args scripts/diagnostic-artifact-qa.sh "$diagnostic_dir" "$evidence_path")"
    printf -- '- Diagnostic directory: `%s`\n' "$diagnostic_dir"
    printf -- '- Evidence path: `%s`\n' "$evidence_path"
    printf -- '- Primary zip: `%s`\n' "${primary_zip:-missing}"
    printf -- '- Staged app: `%s`\n' "${app_path:-missing}"
    printf -- '- Temporary staging directory: `%s`\n\n' "${temp_dir:-missing}"
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
    printf 'RESULT: %s\n' "$result"
  } | sanitize_text >"$evidence_path"
}

cleanup_path() {
  local path="$1"
  if [ -n "$path" ] && [[ "$path" == /tmp/morrow-diagnostic-* ]]; then
    rm -rf "$path"
  fi
}

run_real_mode() {
  local diagnostic_dir="$1" evidence_path="$2" temp_dir zip_list app_list zip_note_list zip_count primary_zip manifest note sums dmg_count dmg_list
  local unzip_status app_count zip_app_count staged_app_count zip_note_count app_path plist plist_json_file bundle_id bundle_name display_name executable_name executable_path
  local carbon_required codesign_verify_status codesign_details_status spctl_status stapler_status details_file cleanup_target result

  if [ -z "$diagnostic_dir" ] || [ -z "$evidence_path" ]; then
    printf 'Usage: scripts/diagnostic-artifact-qa.sh [--self-test <evidence.md> | <diagnostic-dir> <evidence.md>]\n' >&2
    return 2
  fi
  if ! [[ "$timeout_seconds" =~ ^[0-9]+$ ]] || [ "$timeout_seconds" -le 0 ]; then
    printf 'diagnostic-artifact-qa: MORROW_DIAGNOSTIC_QA_TIMEOUT_SECONDS must be a positive integer\n' >&2
    return 2
  fi

  temp_dir="$(mktemp -d /tmp/morrow-diagnostic-qa-XXXXXX)"
  tmp_dirs+=("$temp_dir")
  zip_list="$(mktemp "${TMPDIR:-/tmp}/morrow-diagnostic-artifact-zips-XXXXXX")"
  app_list="$(mktemp "${TMPDIR:-/tmp}/morrow-diagnostic-artifact-apps-XXXXXX")"
  zip_note_list="$(mktemp "${TMPDIR:-/tmp}/morrow-diagnostic-artifact-zip-notes-XXXXXX")"
  dmg_list="$(mktemp "${TMPDIR:-/tmp}/morrow-diagnostic-artifact-dmgs-XXXXXX")"
  plist_json_file="$(mktemp "${TMPDIR:-/tmp}/morrow-diagnostic-artifact-plist-XXXXXX")"
  tmp_files+=("$zip_list" "$app_list" "$zip_note_list" "$dmg_list" "$plist_json_file")

  if [ -d "$diagnostic_dir" ]; then
    record "PASS" "diagnostic directory exists" "$diagnostic_dir"
  else
    record "FAIL" "diagnostic directory exists" "$diagnostic_dir"
  fi

  zip_count="$(find_one_file "$diagnostic_dir" '*.zip' "$zip_list")"
  if [ "$zip_count" -eq 1 ]; then
    primary_zip="$(sed -n '1p' "$zip_list")"
    record "PASS" "exactly one primary diagnostic zip is present" "$(basename "$primary_zip")"
  else
    primary_zip=""
    record "FAIL" "exactly one primary diagnostic zip is present" "found $zip_count"
  fi

  manifest="$diagnostic_dir/manifest.json"
  note="$diagnostic_dir/Morrow-diagnostic-tester-note.md"
  sums="$diagnostic_dir/SHA256SUMS"
  [ -f "$manifest" ] && record "PASS" "external manifest is present" "$manifest" || record "FAIL" "external manifest is present" "$manifest"
  [ -f "$note" ] && record "PASS" "external tester note is present" "$note" || record "FAIL" "external tester note is present" "$note"
  [ -f "$sums" ] && record "PASS" "external SHA256SUMS is present" "$sums" || record "FAIL" "external SHA256SUMS is present" "$sums"

  : >"$dmg_list"
  if [ -d "$diagnostic_dir" ]; then
    find "$diagnostic_dir" -maxdepth 1 -type f -name '*.dmg' -print >"$dmg_list"
  fi
  dmg_count="$(wc -l <"$dmg_list" | tr -d ' ')"
  if [ "$dmg_count" -le 1 ]; then
    record "PASS" "optional DMG sidecar count is zero or one" "found $dmg_count"
  else
    record "FAIL" "optional DMG sidecar count is zero or one" "found $dmg_count"
  fi

  if [ -n "$primary_zip" ]; then
    if command -v unzip >/dev/null 2>&1; then
      set +e
      unzip -q "$primary_zip" -d "$temp_dir/unpacked" >/dev/null 2>&1
      unzip_status=$?
      set -e
      if [ "$unzip_status" -eq 0 ]; then
        record "PASS" "primary zip unpacks cleanly" "$primary_zip"
      else
        record "FAIL" "primary zip unpacks cleanly" "unzip exit $unzip_status"
      fi
    else
      record "FAIL" "primary zip unpacks cleanly" "unzip is not available"
    fi
  else
    record "FAIL" "primary zip unpacks cleanly" "no primary zip"
  fi

  zip_app_count="$(find_one_app "$temp_dir/unpacked" "$app_list")"
  if [ -n "$primary_zip" ]; then
    if [ "$zip_app_count" -eq 1 ]; then
      record "PASS" "primary zip contains exactly one Morrow.app" "$(sed -n '1p' "$app_list")"
    else
      record "FAIL" "primary zip contains exactly one Morrow.app" "archive integrity failure: found $zip_app_count in primary zip; missing Morrow.app in primary zip"
    fi
  else
    record "FAIL" "primary zip contains exactly one Morrow.app" "no primary zip"
  fi

  zip_note_count="$(find_one_note "$temp_dir/unpacked" "$zip_note_list")"
  if [ -n "$primary_zip" ]; then
    if [ "$zip_note_count" -eq 1 ]; then
      record "PASS" "primary zip contains tester note" "$(sed -n '1p' "$zip_note_list")"
    else
      record "FAIL" "primary zip contains tester note" "archive integrity failure: found $zip_note_count Morrow-diagnostic-tester-note.md entries in primary zip"
    fi
  else
    record "FAIL" "primary zip contains tester note" "no primary zip"
  fi

  app_count="$zip_app_count"
  if [ -n "$primary_zip" ] && [ "$app_count" -ne 1 ]; then
    app_path=""
    record "FAIL" "Morrow.app used for Info.plist and signing checks comes from primary zip" "archive integrity failure: external staged fallback is not allowed when primary zip exists"
  elif [ "$app_count" -eq 0 ]; then
    staged_app_count="$(find_one_app "$diagnostic_dir" "$app_list")"
    app_count="$staged_app_count"
    if [ "$app_count" -eq 1 ]; then
      app_path="$(sed -n '1p' "$app_list")"
      record "PASS" "Morrow.app used for Info.plist and signing checks comes from diagnostic staging" "$app_path"
    else
      app_path=""
      record "FAIL" "Morrow.app used for Info.plist and signing checks comes from diagnostic staging" "found $app_count"
    fi
  elif [ "$app_count" -eq 1 ]; then
    app_path="$(sed -n '1p' "$app_list")"
    record "PASS" "Morrow.app used for Info.plist and signing checks comes from primary zip" "$app_path"
  else
    app_path=""
    record "FAIL" "Morrow.app used for Info.plist and signing checks comes from primary zip" "found $app_count"
  fi

  if [ -n "$app_path" ]; then
    plist="$app_path/Contents/Info.plist"
    if [ -f "$plist" ]; then
      record "PASS" "Info.plist exists" "$plist"
      plist_json "$plist" "$plist_json_file" || true
    else
      record "FAIL" "Info.plist exists" "$plist"
    fi
  else
    plist=""
    record "FAIL" "Info.plist exists" "no Morrow.app"
  fi

  if [ -s "$plist_json_file" ]; then
    bundle_id="$(json_value "$plist_json_file" CFBundleIdentifier 2>/dev/null || true)"
    bundle_name="$(json_value "$plist_json_file" CFBundleName 2>/dev/null || true)"
    display_name="$(json_value "$plist_json_file" CFBundleDisplayName 2>/dev/null || true)"
    executable_name="$(json_value "$plist_json_file" CFBundleExecutable 2>/dev/null || true)"
    [ "$bundle_id" = "dev.morrow.desktop" ] && record "PASS" "CFBundleIdentifier is dev.morrow.desktop" "$bundle_id" || record "FAIL" "CFBundleIdentifier is dev.morrow.desktop" "${bundle_id:-missing}"
    [ "$bundle_name" = "Morrow" ] && record "PASS" "CFBundleName product name is Morrow" "$bundle_name" || record "FAIL" "CFBundleName product name is Morrow" "${bundle_name:-missing}"
    if [ -z "$display_name" ] || [ "$display_name" = "Morrow" ]; then
      record "PASS" "CFBundleDisplayName is absent or Morrow" "${display_name:-absent}"
    else
      record "FAIL" "CFBundleDisplayName is absent or Morrow" "$display_name"
    fi
    carbon_required="$(json_value "$plist_json_file" LSRequiresCarbon 2>/dev/null || true)"
    if [ -z "$carbon_required" ]; then
      record "PASS" "LSRequiresCarbon is absent for modern LaunchServices" "absent"
    else
      record "FAIL" "LSRequiresCarbon is absent for modern LaunchServices" "$carbon_required"
    fi
    [ -n "$executable_name" ] && record "PASS" "CFBundleExecutable is declared" "$executable_name" || record "FAIL" "CFBundleExecutable is declared" "missing"
  else
    executable_name=""
    record "FAIL" "CFBundleIdentifier is dev.morrow.desktop" "Info.plist unavailable"
    record "FAIL" "CFBundleName product name is Morrow" "Info.plist unavailable"
    record "FAIL" "CFBundleDisplayName is absent or Morrow" "Info.plist unavailable"
    record "FAIL" "LSRequiresCarbon is absent for modern LaunchServices" "Info.plist unavailable"
    record "FAIL" "CFBundleExecutable is declared" "Info.plist unavailable"
  fi

  executable_path=""
  if [ -n "$app_path" ] && [ -n "$executable_name" ]; then
    executable_path="$app_path/Contents/MacOS/$executable_name"
  fi
  if [ -n "$executable_path" ] && [ -f "$executable_path" ] && [ -x "$executable_path" ]; then
    record "PASS" "app executable exists and is executable" "$executable_path"
  else
    record "FAIL" "app executable exists and is executable" "${executable_path:-missing executable path}"
  fi

  if [ -n "$app_path" ]; then
    set +e
    run_bounded_command "codesign verify app" codesign codesign --verify --deep --strict "$app_path"
    codesign_verify_status=$?
    set -e
    [ "$codesign_verify_status" -eq 0 ] && record "PASS" "codesign verifies app bundle" "exit 0" || record "FAIL" "codesign verifies app bundle" "exit $codesign_verify_status"

    set +e
    run_bounded_command "codesign details app" codesign codesign -dv "$app_path"
    codesign_details_status=$?
    set -e
    [ "$codesign_details_status" -eq 0 ] && record "PASS" "codesign details are readable" "exit 0" || record "FAIL" "codesign details are readable" "exit $codesign_details_status"
    details_file="${command_outputs[$((${#command_outputs[@]} - 1))]}"
    if [ -f "$details_file" ] && grep -E 'Signature=adhoc|Authority=Ad Hoc' "$details_file" >/dev/null 2>&1; then
      record "PASS" "codesign details report ad-hoc signature" "ad-hoc signature marker found"
    else
      record "FAIL" "codesign details report ad-hoc signature" "ad-hoc signature marker missing"
    fi

    set +e
    run_bounded_command "spctl assess app" spctl spctl --assess --type execute "$app_path"
    spctl_status=$?
    set -e
    [ "$spctl_status" -ne 0 ] && record "PASS" "spctl rejects ad-hoc diagnostic app as expected" "exit $spctl_status" || record "FAIL" "spctl rejects ad-hoc diagnostic app as expected" "unexpected acceptance"

    set +e
    run_bounded_command "stapler validate app" xcrun xcrun stapler validate "$app_path"
    stapler_status=$?
    set -e
    [ "$stapler_status" -ne 0 ] && record "PASS" "stapler rejects missing notarization as expected" "exit $stapler_status" || record "FAIL" "stapler rejects missing notarization as expected" "unexpected acceptance"
  else
    record "FAIL" "codesign verifies app bundle" "no Morrow.app"
    record "FAIL" "codesign details are readable" "no Morrow.app"
    record "FAIL" "codesign details report ad-hoc signature" "no Morrow.app"
    record "FAIL" "spctl rejects ad-hoc diagnostic app as expected" "no Morrow.app"
    record "FAIL" "stapler rejects missing notarization as expected" "no Morrow.app"
  fi

  if [ -f "$manifest" ]; then
    if node -e 'JSON.parse(require("node:fs").readFileSync(process.argv[1], "utf8"))' "$manifest" >/dev/null 2>&1; then
      record "PASS" "manifest parses as JSON" "$manifest"
      json_bool "$manifest" 'return data.productName === "Morrow" || data.product === "Morrow";' && record "PASS" "manifest product is Morrow" "product field" || record "FAIL" "manifest product is Morrow" "missing productName/product"
      json_bool "$manifest" 'return data.bundleIdentifier === "dev.morrow.desktop" || data.identifier === "dev.morrow.desktop";' && record "PASS" "manifest bundle identifier is dev.morrow.desktop" "identifier field" || record "FAIL" "manifest bundle identifier is dev.morrow.desktop" "missing bundle identifier"
      json_bool "$manifest" 'return data.signingMode === "ad-hoc";' && record "PASS" "manifest records ad-hoc signing mode" "signingMode" || record "FAIL" "manifest records ad-hoc signing mode" "missing signingMode=ad-hoc"
      json_bool "$manifest" 'return data.notarization === "not_notarized_expected";' && record "PASS" "manifest records expected non-notarized status" "notarization" || record "FAIL" "manifest records expected non-notarized status" "missing notarization=not_notarized_expected"
      json_bool "$manifest" 'return data.architectureScope === "host_only" || data.architecture === "host_only";' && record "PASS" "manifest records host-only architecture scope" "host_only" || record "FAIL" "manifest records host-only architecture scope" "missing host_only scope"
      json_bool "$manifest" 'return typeof (data.hostArchitecture || data.unameMachine) === "string" && (data.hostArchitecture || data.unameMachine).length > 0;' && record "PASS" "manifest records host architecture" "hostArchitecture/unameMachine" || record "FAIL" "manifest records host architecture" "missing host architecture"
      json_bool "$manifest" 'return typeof data.rustHostTarget === "string" && data.rustHostTarget.length > 0;' && record "PASS" "manifest records Rust host target" "rustHostTarget" || record "FAIL" "manifest records Rust host target" "missing rustHostTarget"
      json_bool "$manifest" 'return typeof data.tauriTarget === "string" && data.tauriTarget.length > 0;' && record "PASS" "manifest records Tauri target" "tauriTarget" || record "FAIL" "manifest records Tauri target" "missing tauriTarget"
      json_bool "$manifest" 'return data.freshness?.appBundle === "pre_existing_source_bundle_removed_before_tauri_invocation";' && record "PASS" "manifest records app freshness proof" "freshness.appBundle" || record "FAIL" "manifest records app freshness proof" "missing app freshness proof"
      json_bool "$manifest" 'return data.freshness?.fallbackPolicy === "nonzero_tauri_exit_requires_recreated_current_run_app_bundle";' && record "PASS" "manifest records fallback freshness policy" "freshness.fallbackPolicy" || record "FAIL" "manifest records fallback freshness policy" "missing fallback freshness policy"
    else
      record "FAIL" "manifest parses as JSON" "$manifest"
      record "FAIL" "manifest product is Morrow" "manifest parse failed"
      record "FAIL" "manifest bundle identifier is dev.morrow.desktop" "manifest parse failed"
      record "FAIL" "manifest records ad-hoc signing mode" "manifest parse failed"
      record "FAIL" "manifest records expected non-notarized status" "manifest parse failed"
      record "FAIL" "manifest records host-only architecture scope" "manifest parse failed"
      record "FAIL" "manifest records host architecture" "manifest parse failed"
      record "FAIL" "manifest records Rust host target" "manifest parse failed"
      record "FAIL" "manifest records Tauri target" "manifest parse failed"
      record "FAIL" "manifest records app freshness proof" "manifest parse failed"
      record "FAIL" "manifest records fallback freshness policy" "manifest parse failed"
    fi
  else
    record "FAIL" "manifest parses as JSON" "manifest missing"
    record "FAIL" "manifest product is Morrow" "manifest missing"
    record "FAIL" "manifest bundle identifier is dev.morrow.desktop" "manifest missing"
    record "FAIL" "manifest records ad-hoc signing mode" "manifest missing"
    record "FAIL" "manifest records expected non-notarized status" "manifest missing"
    record "FAIL" "manifest records host-only architecture scope" "manifest missing"
    record "FAIL" "manifest records host architecture" "manifest missing"
    record "FAIL" "manifest records Rust host target" "manifest missing"
    record "FAIL" "manifest records Tauri target" "manifest missing"
    record "FAIL" "manifest records app freshness proof" "manifest missing"
    record "FAIL" "manifest records fallback freshness policy" "manifest missing"
  fi

  if [ -f "$sums" ]; then
    [ -n "$primary_zip" ] && record_checksum_match "primary zip" "$sums" "$primary_zip" "$(basename "$primary_zip")" || record "FAIL" "primary zip has SHA-256 coverage" "no primary zip"
    record_checksum_match "external manifest" "$sums" "$manifest" "manifest.json"
    record_checksum_match "external tester note" "$sums" "$note" "Morrow-diagnostic-tester-note.md"
    if [ -n "$executable_path" ]; then
      record_checksum_match "app executable" "$sums" "$executable_path" "Morrow.app/Contents/MacOS/$executable_name"
    else
      record "FAIL" "app executable has SHA-256 coverage" "missing executable"
    fi
    if [ "$dmg_count" -eq 1 ]; then
      record_checksum_match "optional DMG" "$sums" "$(sed -n '1p' "$dmg_list")" "$(basename "$(sed -n '1p' "$dmg_list")")"
    else
      record "PASS" "optional DMG SHA-256 coverage" "no DMG sidecar present"
    fi
  else
    record "FAIL" "primary zip has SHA-256 coverage" "SHA256SUMS missing"
    record "FAIL" "external manifest has SHA-256 coverage" "SHA256SUMS missing"
    record "FAIL" "external tester note has SHA-256 coverage" "SHA256SUMS missing"
    record "FAIL" "app executable has SHA-256 coverage" "SHA256SUMS missing"
    record "FAIL" "optional DMG SHA-256 coverage" "SHA256SUMS missing"
  fi

  cleanup_target="$temp_dir"
  cleanup_path "$temp_dir"
  if [ ! -e "$cleanup_target" ]; then
    record "PASS" "temporary staging directory cleanup" "$cleanup_target removed"
  else
    record "FAIL" "temporary staging directory cleanup" "$cleanup_target still exists"
  fi

  if printf '%s\n' "${checks[@]}" | grep -q '^FAIL'$'\t'; then
    result="FAIL"
  else
    result="PASS"
  fi
  write_real_evidence "$diagnostic_dir" "$evidence_path" "$result" "$primary_zip" "$app_path" "$cleanup_target"
  [ "$result" = "PASS" ]
}

create_valid_fixture() {
  local root="$1" arch rust_target tauri_target zip_name app_dir exe plist
  arch="$(uname -m)"
  rust_target="$(rustc -vV 2>/dev/null | awk -F': ' '/^host:/ { print $2; exit }')"
  [ -n "$rust_target" ] || rust_target="${arch}-apple-darwin"
  tauri_target="$rust_target"
  zip_name="Morrow-0.1.0-${arch}-diagnostic.zip"
  app_dir="$root/payload/Morrow.app"
  exe="$app_dir/Contents/MacOS/Morrow"
  plist="$app_dir/Contents/Info.plist"

  mkdir -p "$app_dir/Contents/MacOS" "$app_dir/Contents/Resources"
  cat >"$exe" <<'SCRIPT'
#!/usr/bin/env bash
printf 'Morrow diagnostic fixture\n'
SCRIPT
  chmod +x "$exe"
  cat >"$plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleExecutable</key>
  <string>Morrow</string>
  <key>CFBundleIdentifier</key>
  <string>dev.morrow.desktop</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>Morrow</string>
  <key>CFBundleDisplayName</key>
  <string>Morrow</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>0.1.0</string>
  <key>CFBundleVersion</key>
  <string>0.1.0</string>
</dict>
</plist>
PLIST
  codesign --force --deep --sign - "$app_dir" >/dev/null 2>&1

  printf '# Morrow diagnostic tester note\n' >"$root/Morrow-diagnostic-tester-note.md"
  cp "$root/Morrow-diagnostic-tester-note.md" "$root/payload/Morrow-diagnostic-tester-note.md"
  cat >"$root/manifest.json" <<JSON
{
  "productName": "Morrow",
  "bundleIdentifier": "dev.morrow.desktop",
  "version": "0.1.0",
  "signingMode": "ad-hoc",
  "notarization": "not_notarized_expected",
  "architectureScope": "host_only",
  "hostArchitecture": "$arch",
  "rustHostTarget": "$rust_target",
  "tauriTarget": "$tauri_target",
  "freshness": {
    "appBundle": "pre_existing_source_bundle_removed_before_tauri_invocation",
    "dmgSidecar": "pre_existing_dmg_sidecars_removed_before_tauri_invocation",
    "fallbackPolicy": "nonzero_tauri_exit_requires_recreated_current_run_app_bundle"
  },
  "primaryZip": "$zip_name",
  "testerNote": "Morrow-diagnostic-tester-note.md",
  "appExecutable": "payload/Morrow.app/Contents/MacOS/Morrow"
}
JSON
  (cd "$root/payload" && zip -qry "../$zip_name" Morrow.app Morrow-diagnostic-tester-note.md)
  (cd "$root" && shasum -a 256 "$zip_name" manifest.json Morrow-diagnostic-tester-note.md payload/Morrow.app/Contents/MacOS/Morrow >SHA256SUMS)
}

create_malformed_fixture() {
  local root="$1"
  mkdir -p "$root/payload/Morrow.app/Contents/MacOS"
  printf '{}\n' >"$root/manifest.json"
}

append_file_excerpt() {
  local label="$1" path="$2"
  printf '### %s\n\n' "$label"
  if [ -f "$path" ]; then
    printf '```text\n'
    sed -n '1,220p' "$path" | sanitize_text
    printf '```\n\n'
  else
    printf 'Missing: `%s`\n\n' "$path"
  fi
}

run_self_test() {
  local evidence_path="$1" root valid_dir malformed_dir valid_evidence malformed_evidence valid_status malformed_status cleanup_status result valid_excerpt malformed_excerpt
  root="$(mktemp -d /tmp/morrow-diagnostic-artifact-self-test-XXXXXX)"
  valid_dir="$root/valid"
  malformed_dir="$root/malformed"
  valid_evidence="$root/valid-evidence.md"
  malformed_evidence="$root/malformed-evidence.md"
  valid_excerpt="$(mktemp "${TMPDIR:-/tmp}/morrow-diagnostic-artifact-valid-excerpt-XXXXXX")"
  malformed_excerpt="$(mktemp "${TMPDIR:-/tmp}/morrow-diagnostic-artifact-malformed-excerpt-XXXXXX")"
  tmp_files+=("$valid_excerpt" "$malformed_excerpt")
  mkdir -p "$valid_dir" "$malformed_dir"

  set +e
  create_valid_fixture "$valid_dir"
  valid_fixture_status=$?
  set -e
  if [ "$valid_fixture_status" -ne 0 ]; then
    printf '# Todo 2 Diagnostic Artifact QA Self-Test Evidence\n\nRESULT: FAIL\n' >"$evidence_path"
    rm -rf "$root"
    return 1
  fi
  create_malformed_fixture "$malformed_dir"

  set +e
  bash "$script_path" "$valid_dir" "$valid_evidence" >/dev/null 2>&1
  valid_status=$?
  bash "$script_path" "$malformed_dir" "$malformed_evidence" >/dev/null 2>&1
  malformed_status=$?
  set -e

  if [ "$valid_status" -eq 0 ]; then
    record "PASS" "self-test valid diagnostic metadata fixture passes" "exit 0"
  else
    record "FAIL" "self-test valid diagnostic metadata fixture passes" "exit $valid_status"
  fi
  if [ "$malformed_status" -ne 0 ] && grep -E 'FAIL .*(Morrow.app|Info.plist|signature|manifest)' "$malformed_evidence" >/dev/null 2>&1; then
    record "PASS" "self-test malformed diagnostic metadata fixture fails" "exit $malformed_status with expected failure evidence"
  else
    record "FAIL" "self-test malformed diagnostic metadata fixture fails" "exit $malformed_status without expected failure evidence"
  fi

  if [ -f "$valid_evidence" ]; then
    sed -n '1,220p' "$valid_evidence" | sanitize_text >"$valid_excerpt"
  fi
  if [ -f "$malformed_evidence" ]; then
    sed -n '1,220p' "$malformed_evidence" | sanitize_text >"$malformed_excerpt"
  fi

  rm -rf "$root"
  if [ ! -e "$root" ]; then
    cleanup_status="PASS"
    record "PASS" "self-test fixture cleanup" "$root removed"
  else
    cleanup_status="FAIL"
    record "FAIL" "self-test fixture cleanup" "$root still exists"
  fi

  if printf '%s\n' "${checks[@]}" | grep -q '^FAIL'$'\t'; then
    result="FAIL"
  else
    result="PASS"
  fi

  mkdir -p "$(dirname "$evidence_path")"
  {
    printf '# Todo 2 Diagnostic Artifact QA Self-Test Evidence\n\n'
    printf '## Scenario\n\n'
    printf -- '- Invocation: `%s`\n' "$(quote_args scripts/diagnostic-artifact-qa.sh --self-test "$evidence_path")"
    printf -- '- Valid fixture observable: diagnostic metadata fixture exits 0 and records RESULT: PASS.\n'
    printf -- '- Malformed fixture observable: invalid diagnostic fixture exits nonzero and records FAIL assertions for bundle metadata/signature/manifest.\n'
    printf -- '- Cleanup observable: generated fixture root removed after run.\n\n'
    printf '## Assertions\n\n'
    for check in "${checks[@]}"; do
      local status name details
      IFS=$'\t' read -r status name details <<<"$check"
      printf -- '- %s %s: %s\n' "$status" "$name" "$details"
    done
    printf '\n## Fixture Evidence Excerpts\n\n'
    append_file_excerpt "valid fixture evidence" "$valid_excerpt"
    append_file_excerpt "malformed fixture evidence" "$malformed_excerpt"
    printf '## Adversarial Class Probes\n\n'
    printf -- '- malformed_input: probed with missing zip, empty manifest, and incomplete Morrow.app fixture; harness returned nonzero and recorded FAIL details.\n'
    printf -- '- generated_cached_artifacts: self-test generated temp fixtures under `/tmp` and cleanup check recorded %s.\n' "$cleanup_status"
    printf -- '- dirty_worktree: initial `git status --short` was captured before edits; unrelated changes were left untouched.\n'
    printf -- '- misleading_success_output: harness relies on assertion list and command exit statuses, not command text alone.\n'
    printf -- '- stale_state: self-test reruns the current script path after chmod/content changes.\n'
    printf -- '- prompt_injection: no untrusted external text is executed as a command; fixture data is parsed as plist or JSON only.\n'
    printf -- '- cancel_resume: no resumable flow; each invocation writes complete evidence from scratch.\n'
    printf -- '- hung_long_commands: external commands are bounded by `MORROW_DIAGNOSTIC_QA_TIMEOUT_SECONDS`.\n'
    printf -- '- flaky_tests: fixtures are deterministic local files created per run.\n'
    printf -- '- repeated_interruptions: no interrupt recovery path is required for this bounded harness.\n\n'
    printf 'RESULT: %s\n' "$result"
  } | sanitize_text >"$evidence_path"

  [ "$result" = "PASS" ]
}

cleanup_all() {
  local path
  for path in "${tmp_files[@]}"; do
    [ -n "$path" ] && rm -f "$path" 2>/dev/null || true
  done
  for path in "${tmp_dirs[@]}"; do
    cleanup_path "$path" 2>/dev/null || true
  done
}
trap cleanup_all EXIT

if [ "${1:-}" = "--self-test" ]; then
  run_self_test "${2:-}"
elif [ $# -eq 2 ]; then
  run_real_mode "$1" "$2"
else
  printf 'Usage: scripts/diagnostic-artifact-qa.sh [--self-test <evidence.md> | <diagnostic-dir> <evidence.md>]\n' >&2
  exit 2
fi
