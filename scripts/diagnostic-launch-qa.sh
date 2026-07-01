#!/usr/bin/env bash
# noqa: SIZE_OK - single-purpose diagnostic launch QA keeps GUI process ownership, screenshot proof, and cleanup reporting together; split if it grows further.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

timeout_seconds="${MORROW_DIAGNOSTIC_QA_TIMEOUT_SECONDS:-60}"
original_args=("$@")
checks=()
command_labels=()
command_commands=()
command_statuses=()
command_outputs=()
tmp_files=()
temp_dir=""
cleanup_receipts=()
launched_pids=()
app_path=""

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
  output_file="$(mktemp "${TMPDIR:-/tmp}/morrow-diagnostic-launch-command-XXXXXX")"
  timeout_file="$(mktemp "${TMPDIR:-/tmp}/morrow-diagnostic-launch-timeout-XXXXXX")"
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

pids_for_app_path() {
  local marker private_marker
  [ -n "$app_path" ] || return 0
  marker="$app_path/Contents/MacOS/"
  private_marker="/private${marker}"
  { ps -axo pid=,args= 2>/dev/null || true; } | awk -v marker="$marker" -v private_marker="$private_marker" '
    {
      pid = $1
      $1 = ""
      sub(/^[[:space:]]+/, "", $0)
      if (index($0, "awk -v marker=") == 0 && (index($0, marker) == 1 || index($0, private_marker) == 1 || index($0, "bash " marker) > 0 || index($0, "bash " private_marker) > 0 || index($0, "sh " marker) > 0 || index($0, "sh " private_marker) > 0 || index($0, "zsh " marker) > 0 || index($0, "zsh " private_marker) > 0)) {
        print pid
      }
    }
  '
}

wait_for_app_pids() {
  local deadline now pids
  deadline=$((SECONDS + timeout_seconds))
  while [ "$SECONDS" -le "$deadline" ]; do
    pids="$(pids_for_app_path | tr '\n' ' ' | sed 's/[[:space:]]*$//')"
    if [ -n "$pids" ]; then
      printf '%s' "$pids"
      return 0
    fi
    sleep 1
  done
  return 1
}

wait_for_no_app_pids() {
  local deadline pids
  deadline=$((SECONDS + 20))
  while [ "$SECONDS" -le "$deadline" ]; do
    pids="$(pids_for_app_path | tr '\n' ' ' | sed 's/[[:space:]]*$//')"
    [ -z "$pids" ] && return 0
    sleep 1
  done
  return 1
}

process_identity() {
  local pids="$1" pid
  for pid in $pids; do
    ps -p "$pid" -o pid=,ppid=,comm=,args= 2>/dev/null || true
  done
}

window_probe_for_pid() {
  local pid="$1"
  osascript - "$pid" "$app_path" <<'APPLESCRIPT'
on boolText(value)
  if value then
    return "true"
  end if
  return "false"
end boolText

on safeText(value)
  try
    if value is missing value then
      return ""
    end if
    return value as text
  on error
    return ""
  end try
end safeText

on run argv
  set targetPid to (item 1 of argv) as integer
  set appPath to item 2 of argv
  set linefeedText to ASCII character 10
  set outputText to "pid=" & targetPid & linefeedText
  try
    tell application (POSIX file appPath as alias) to activate
  end try
  delay 0.5
  tell application "System Events"
    set matchingProcesses to application processes whose unix id is targetPid
    if (count of matchingProcesses) is 0 then
      return outputText & "status=missing_process"
    end if

    set targetProcess to item 1 of matchingProcesses
    try
      set frontmost of targetProcess to true
    end try
    delay 0.25

    set processName to my safeText(name of targetProcess)
    set isFrontmost to frontmost of targetProcess
    set windowCount to count of windows of targetProcess
    set visibleWindowCount to 0
    set boundedWindowCount to 0
    set outputText to outputText & "process_name=" & processName & linefeedText
    set outputText to outputText & "frontmost=" & my boolText(isFrontmost) & linefeedText
    set outputText to outputText & "window_count=" & windowCount & linefeedText

    repeat with windowIndex from 1 to windowCount
      set targetWindow to window windowIndex of targetProcess
      set windowTitle to my safeText(name of targetWindow)
      set windowVisible to "unavailable"
      set windowPosition to ""
      set windowSize to ""
      try
        set windowVisible to visible of targetWindow
      end try
      try
        set {windowX, windowY} to position of targetWindow
        set windowPosition to windowX & "," & windowY
      end try
      try
        set {windowWidth, windowHeight} to size of targetWindow
        set windowSize to windowWidth & "x" & windowHeight
        if windowWidth > 0 and windowHeight > 0 then
          set boundedWindowCount to boundedWindowCount + 1
          if windowVisible is true then
            set visibleWindowCount to visibleWindowCount + 1
          end if
        end if
      end try
      set outputText to outputText & "window_" & windowIndex & "_title=" & windowTitle & linefeedText
      if windowVisible is "unavailable" then
        set outputText to outputText & "window_" & windowIndex & "_visible=unavailable" & linefeedText
      else
        set outputText to outputText & "window_" & windowIndex & "_visible=" & my boolText(windowVisible) & linefeedText
      end if
      set outputText to outputText & "window_" & windowIndex & "_position=" & windowPosition & linefeedText
      set outputText to outputText & "window_" & windowIndex & "_size=" & windowSize & linefeedText
    end repeat

    set outputText to outputText & "visible_window_count=" & visibleWindowCount & linefeedText
    set outputText to outputText & "bounded_window_count=" & boundedWindowCount & linefeedText
    if boundedWindowCount > 0 and isFrontmost then
      return outputText & "status=frontmost_window_with_bounds"
    end if
    if boundedWindowCount > 0 then
      return outputText & "status=window_with_bounds"
    end if
    return outputText & "status=no_visible_window"
  end tell
end run
APPLESCRIPT
}

wait_for_window_proof() {
  local pids="$1" output_file="$2" deadline pid probe_output probe_status success window_timeout current_pids probe_pids
  window_timeout="${MORROW_DIAGNOSTIC_WINDOW_PROOF_SECONDS:-90}"
  deadline=$((SECONDS + window_timeout))
  : >"$output_file"
  while [ "$SECONDS" -le "$deadline" ]; do
    : >"$output_file"
    success=1
    current_pids="$(pids_for_app_path | tr '\n' ' ' | sed 's/[[:space:]]*$//')"
    probe_pids="$(printf '%s\n%s\n' "$pids" "$current_pids" | tr ' ' '\n' | awk 'NF && !seen[$0]++' | tr '\n' ' ' | sed 's/[[:space:]]*$//')"
    for pid in $probe_pids; do
      probe_status=0
      probe_output="$(window_probe_for_pid "$pid" 2>&1)" || probe_status=$?
      {
        printf -- '--- window probe for pid %s ---\n' "$pid"
        printf 'app_path=%s\n' "$app_path"
        printf '%s\n' "$probe_output"
        printf '\nprobe_exit=%s\n' "$probe_status"
      } >>"$output_file"
      if [ "$probe_status" -eq 0 ] &&
        printf '%s\n' "$probe_output" | grep -Eq '^status=(frontmost_window_with_bounds|window_with_bounds)$' &&
        printf '%s\n' "$probe_output" | grep -Eq '^bounded_window_count=[1-9][0-9]*$'; then
        success=0
        break
      fi
    done
    [ "$success" -eq 0 ] && return 0
    sleep 1
  done
  return 1
}

quit_launched_processes_with_osascript() {
  local pids="$1" pid status=0
  [ -n "$pids" ] || return 1
  for pid in $pids; do
    if ! osascript - "$pid" <<'APPLESCRIPT'
on run argv
  set targetPid to (item 1 of argv) as integer
  tell application "System Events"
    set matchingProcesses to application processes whose unix id is targetPid
    if (count of matchingProcesses) is 0 then
      return "missing"
    end if
    tell item 1 of matchingProcesses to quit
    return "quit-requested"
  end tell
end run
APPLESCRIPT
    then
      status=1
    fi
  done
  return "$status"
}

cleanup_launched_processes() {
  local pids pid deadline
  pids="$(pids_for_app_path | tr '\n' ' ' | sed 's/[[:space:]]*$//')"
  if [ -n "$pids" ]; then
    if quit_launched_processes_with_osascript "$pids" >/dev/null 2>&1; then
      cleanup_receipts+=("requested osascript quit for leftover launched Morrow PID(s): $pids")
      if wait_for_no_app_pids; then
        return 0
      fi
    fi
    for pid in $pids; do
      kill "$pid" 2>/dev/null || true
      cleanup_receipts+=("sent TERM to leftover launched Morrow PID $pid")
    done
    deadline=$((SECONDS + 10))
    while [ "$SECONDS" -le "$deadline" ]; do
      pids="$(pids_for_app_path | tr '\n' ' ' | sed 's/[[:space:]]*$//')"
      [ -z "$pids" ] && return 0
      sleep 1
    done
  fi
  deadline=$((SECONDS + 10))
  while [ "$SECONDS" -le "$deadline" ]; do
    pids="$(pids_for_app_path | tr '\n' ' ' | sed 's/[[:space:]]*$//')"
    [ -z "$pids" ] && return 0
    for pid in $pids; do
      kill -KILL "$pid" 2>/dev/null || true
      cleanup_receipts+=("sent KILL to leftover launched Morrow PID $pid")
    done
    sleep 1
  done
}

cleanup_temp() {
  cleanup_launched_processes
  if [ -n "$temp_dir" ] && [[ "$temp_dir" == /tmp/morrow-diagnostic-qa-* ]] && [ -e "$temp_dir" ]; then
    rm -rf "$temp_dir"
    cleanup_receipts+=("removed temp launch copy $temp_dir")
  fi
  cleanup_launched_processes
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

screenshot_metadata() {
  local screenshot="$1"
  {
    stat -f 'size_bytes=%z' "$screenshot" 2>/dev/null || true
    sips -g pixelWidth -g pixelHeight "$screenshot" 2>/dev/null || true
    file "$screenshot" 2>/dev/null || true
  }
}

screenshot_is_nonblank() {
  local screenshot="$1" bmp_path="$2"
  [ -s "$screenshot" ] || return 1
  command -v sips >/dev/null 2>&1 || return 1
  command -v python3 >/dev/null 2>&1 || return 1
  sips -s format bmp "$screenshot" --out "$bmp_path" >/dev/null 2>&1 || return 1
  python3 - "$bmp_path" <<'PY'
import struct
import sys
from pathlib import Path

data = Path(sys.argv[1]).read_bytes()
if len(data) < 54 or data[:2] != b"BM":
    raise SystemExit(1)
offset = struct.unpack_from("<I", data, 10)[0]
pixels = data[offset:]
if len(pixels) < 100:
    raise SystemExit(1)
sample = pixels[::max(1, len(pixels) // 200000)]
unique = set(sample)
if len(unique) < 2:
    raise SystemExit(1)
if all(byte == 0 for byte in sample) or all(byte == 255 for byte in sample):
    raise SystemExit(1)
raise SystemExit(0)
PY
}

plist_raw_value() {
  local plist="$1" key="$2" value
  if value="$(plutil -extract "$key" raw -o - "$plist" 2>/dev/null)"; then
    printf '%s' "$value"
  fi
}

mounted_morrow_images() {
  if command -v hdiutil >/dev/null 2>&1; then
    hdiutil info 2>/dev/null | awk 'BEGIN{RS=""} /Morrow|morrow|diagnostic/ { print }'
  fi
}

global_morrow_processes() {
  pgrep -fl 'Morrow|morrow' 2>/dev/null || true
}

write_evidence() {
  local diagnostic_dir="$1" evidence_path="$2" result="$3" primary_zip="${4:-}" screenshot_path="${5:-}" process_text="${6:-}" window_text="${7:-}" post_screenshot_window_text="${8:-}" cleanup_scan mount_scan global_process_scan output index metadata
  mkdir -p "$(dirname "$evidence_path")"
  cleanup_scan="$(find /tmp -maxdepth 1 -name 'morrow-diagnostic-qa-*' -print 2>/dev/null || true)"
  mount_scan="$(mounted_morrow_images || true)"
  global_process_scan="$(global_morrow_processes || true)"
  metadata=""
  if [ -n "$screenshot_path" ] && [ -f "$screenshot_path" ]; then
    metadata="$(screenshot_metadata "$screenshot_path")"
  fi
  {
    printf '# Todo 6 Diagnostic Launch QA Evidence\n\n'
    printf '## Scenario\n\n'
    printf -- '- Invocation: `%s`\n' "$(quote_args "$0" "${original_args[@]}")"
    printf -- '- Diagnostic directory: `%s`\n' "$diagnostic_dir"
    printf -- '- Evidence path: `%s`\n' "$evidence_path"
    printf -- '- Screenshot path: `%s`\n' "${screenshot_path:-missing}"
    printf -- '- Primary zip: `%s`\n' "${primary_zip:-missing}"
    printf -- '- Launched packaged app path: `%s`\n\n' "${app_path:-missing}"
    printf '## Assertions\n\n'
    for check in "${checks[@]}"; do
      local status name details
      IFS=$'\t' read -r status name details <<<"$check"
      printf -- '- %s %s: %s\n' "$status" "$name" "$details"
    done
    printf '\n## Process Identity\n\n```text\n%s\n```\n\n' "${process_text:-missing}"
    printf '## Window Proof Before Screenshot\n\n```text\n%s\n```\n\n' "${window_text:-missing}"
    printf '## Window Proof After Screenshot\n\n```text\n%s\n```\n\n' "${post_screenshot_window_text:-missing}"
    printf '## Screenshot Metadata\n\n```text\n%s\n```\n\n' "${metadata:-missing}"
    printf '## Command Transcripts\n\n'
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
      printf -- '- No launched process or temp directory needed cleanup.\n'
    else
      printf -- '- %s\n' "${cleanup_receipts[@]}"
    fi
    if [ -z "$cleanup_scan" ]; then
      printf -- '- PASS cleanup: no `/tmp/morrow-diagnostic-qa-*` directories remain.\n'
    else
      printf -- '- FAIL cleanup: leftover temp directories remain:\n```text\n%s\n```\n' "$cleanup_scan"
    fi
    if [ -z "$mount_scan" ]; then
      printf -- '- PASS cleanup: no Morrow diagnostic DMG mount remains.\n'
    else
      printf -- '- FAIL cleanup: Morrow diagnostic mount remains:\n```text\n%s\n```\n' "$mount_scan"
    fi
    if [ -z "$global_process_scan" ]; then
      printf -- '- PASS cleanup: no global `Morrow|morrow` processes remain after launch QA.\n'
    else
      printf -- '- INFO global process scan after cleanup:\n```text\n%s\n```\n' "$global_process_scan"
    fi
    printf '\nRESULT: %s\n' "$result"
  } | sanitize_text >"$evidence_path"
}

main() {
  local diagnostic_dir="${1:-}" evidence_path="${2:-}" screenshot_path zip_list app_list zip_count primary_zip unzip_status app_count
  local quarantine_status open_status pids process_text screenshot_status bmp_path quit_status result mount_scan exact_pids_after window_text_path post_window_text_path window_text post_screenshot_window_text window_status post_window_status
  local plist carbon_required executable_name executable_path launch_ready

  if [ -z "$diagnostic_dir" ] || [ -z "$evidence_path" ]; then
    usage
    exit 2
  fi
  if ! [[ "$timeout_seconds" =~ ^[0-9]+$ ]] || [ "$timeout_seconds" -le 0 ]; then
    printf 'diagnostic-launch-qa: MORROW_DIAGNOSTIC_QA_TIMEOUT_SECONDS must be a positive integer\n' >&2
    exit 2
  fi

  process_text=""
  window_text=""
  post_screenshot_window_text=""
  screenshot_path="${MORROW_DIAGNOSTIC_LAUNCH_SCREENSHOT:-${evidence_path%.md}.png}"
  rm -f "$screenshot_path"
  temp_dir="$(mktemp -d /tmp/morrow-diagnostic-qa-XXXXXX)"
  zip_list="$(mktemp "${TMPDIR:-/tmp}/morrow-diagnostic-launch-zips-XXXXXX")"
  app_list="$(mktemp "${TMPDIR:-/tmp}/morrow-diagnostic-launch-apps-XXXXXX")"
  bmp_path="$(mktemp "${TMPDIR:-/tmp}/morrow-diagnostic-launch-screenshot-XXXXXX.bmp")"
  window_text_path="$(mktemp "${TMPDIR:-/tmp}/morrow-diagnostic-launch-window-XXXXXX")"
  post_window_text_path="$(mktemp "${TMPDIR:-/tmp}/morrow-diagnostic-launch-post-window-XXXXXX")"
  tmp_files+=("$zip_list" "$app_list" "$bmp_path" "$window_text_path" "$post_window_text_path")

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
    record "FAIL" "diagnostic payload has exactly one primary zip" "found $zip_count; Morrow.app must come from packaged diagnostic payload"
  fi

  if [ -n "$primary_zip" ]; then
    set +e
    run_bounded_command "unpack primary diagnostic zip" unzip unzip -q "$primary_zip" -d "$temp_dir/unpacked"
    unzip_status=$?
    set -e
    [ "$unzip_status" -eq 0 ] && record "PASS" "primary zip unpacks into temp launch copy" "$temp_dir/unpacked" || record "FAIL" "primary zip unpacks into temp launch copy" "unzip exit $unzip_status"
  else
    record "FAIL" "primary zip unpacks into temp launch copy" "no primary zip"
  fi

  app_count="$(find_unpacked_app "$temp_dir/unpacked" "$app_list")"
  if [ "$app_count" -eq 1 ]; then
    app_path="$(sed -n '1p' "$app_list")"
    record "PASS" "Morrow.app comes from packaged diagnostic zip temp copy" "$app_path"
  else
    app_path=""
    record "FAIL" "Morrow.app comes from packaged diagnostic zip temp copy" "found $app_count; diagnostic payload is malformed"
  fi

  if [ -n "$app_path" ]; then
    launch_ready=0
    plist="$app_path/Contents/Info.plist"
    if [ -f "$plist" ]; then
      record "PASS" "launch app Info.plist exists" "$plist"
      carbon_required="$(plist_raw_value "$plist" LSRequiresCarbon)"
      if [ -z "$carbon_required" ]; then
        record "PASS" "launch app LSRequiresCarbon is absent for windowed LaunchServices" "absent"
      else
        record "FAIL" "launch app LSRequiresCarbon is absent for windowed LaunchServices" "$carbon_required"
      fi
      executable_name="$(plist_raw_value "$plist" CFBundleExecutable)"
      if [ -n "$executable_name" ]; then
        record "PASS" "launch app CFBundleExecutable is declared" "$executable_name"
      else
        record "FAIL" "launch app CFBundleExecutable is declared" "missing"
      fi
    else
      executable_name=""
      record "FAIL" "launch app Info.plist exists" "$plist"
      record "FAIL" "launch app LSRequiresCarbon is absent for windowed LaunchServices" "Info.plist missing"
      record "FAIL" "launch app CFBundleExecutable is declared" "Info.plist missing"
    fi

    executable_path=""
    if [ -n "$executable_name" ]; then
      executable_path="$app_path/Contents/MacOS/$executable_name"
    fi
    if [ -n "$executable_path" ] && [ -f "$executable_path" ] && [ -x "$executable_path" ]; then
      record "PASS" "launch app executable exists and is executable before open" "$executable_path"
      launch_ready=1
    else
      record "FAIL" "launch app executable exists and is executable before open" "${executable_path:-missing executable path}"
    fi

    set +e
    run_bounded_command "check temp launch copy quarantine xattr" xattr xattr -p com.apple.quarantine "$app_path"
    quarantine_status=$?
    set -e
    if [ "$quarantine_status" -ne 0 ]; then
      record "PASS" "launch copy is non-quarantined before open" "no com.apple.quarantine attribute on temp app copy"
    else
      record "FAIL" "launch copy is non-quarantined before open" "quarantine attribute present; launch QA requires GUI-approved or non-quarantined payload"
    fi

    if [ "$launch_ready" -eq 1 ]; then
      set +e
      run_bounded_command "open packaged diagnostic Morrow.app" open open -n -F "$app_path"
      open_status=$?
      set -e
      [ "$open_status" -eq 0 ] && record "PASS" "open -n accepts packaged diagnostic Morrow.app" "exit 0" || record "FAIL" "open -n accepts packaged diagnostic Morrow.app" "exit $open_status; Gatekeeper blocking is failure for launch QA unless approval is evidenced"
    else
      open_status=1
      record "FAIL" "open -n accepts packaged diagnostic Morrow.app" "skipped because executable integrity check failed"
    fi

    if [ "$open_status" -eq 0 ] && pids="$(wait_for_app_pids)"; then
      sleep 2
      current_pids="$(pids_for_app_path | tr '\n' ' ' | sed 's/[[:space:]]*$//')"
      [ -n "$current_pids" ] && pids="$current_pids"
      launched_pids=($pids)
      process_text="$(process_identity "$pids")"
      record "PASS" "packaged Morrow process starts from temp app path" "pid(s): $pids"
      if printf '%s\n' "$process_text" | grep -F "$app_path/Contents/MacOS/" >/dev/null 2>&1; then
        record "PASS" "process identity records packaged app path" "process args contain temp packaged app executable"
      else
        record "FAIL" "process identity records packaged app path" "process args missing $app_path/Contents/MacOS"
      fi
    else
      pids=""
      process_text=""
      record "FAIL" "packaged Morrow process starts from temp app path" "no process observed within ${timeout_seconds}s"
      record "FAIL" "process identity records packaged app path" "no process identity"
    fi

    if [ -n "$pids" ]; then
      set +e
      wait_for_window_proof "$pids" "$window_text_path"
      window_status=$?
      set -e
      window_text="$(cat "$window_text_path")"
      if [ "$window_status" -eq 0 ]; then
        record "PASS" "packaged Morrow owns a PID-tied window with bounds before screenshot" "$(grep -E '^(pid|process_name|frontmost|window_1_title|window_1_visible|window_1_position|window_1_size|visible_window_count|bounded_window_count|status)=' "$window_text_path" | tr '\n' '; ')"
      else
        record "FAIL" "packaged Morrow owns a PID-tied window with bounds before screenshot" "no PID-tied Morrow window with positive bounds within ${MORROW_DIAGNOSTIC_WINDOW_PROOF_SECONDS:-90}s"
      fi

      set +e
      run_bounded_command "capture launch screenshot" screencapture screencapture -x "$screenshot_path"
      screenshot_status=$?
      set -e
      if [ "$screenshot_status" -eq 0 ] && [ -s "$screenshot_path" ]; then
        record "PASS" "screencapture creates screenshot artifact" "$screenshot_path"
      else
        record "FAIL" "screencapture creates screenshot artifact" "exit $screenshot_status or empty file"
      fi

      if screenshot_is_nonblank "$screenshot_path" "$bmp_path"; then
        record "PASS" "screenshot artifact is nonblank by pixel probe" "$(screenshot_metadata "$screenshot_path" | tr '\n' '; ')"
      else
        record "FAIL" "screenshot artifact is nonblank by pixel probe" "pixel probe failed or metadata unavailable"
      fi

      set +e
      wait_for_window_proof "$pids" "$post_window_text_path"
      post_window_status=$?
      set -e
      post_screenshot_window_text="$(cat "$post_window_text_path")"
      if [ "$post_window_status" -eq 0 ]; then
        record "PASS" "packaged Morrow window remains PID-tied with bounds after screenshot" "$(grep -E '^(pid|process_name|frontmost|window_1_title|window_1_visible|window_1_position|window_1_size|visible_window_count|bounded_window_count|status)=' "$post_window_text_path" | tr '\n' '; ')"
      else
        record "FAIL" "packaged Morrow window remains PID-tied with bounds after screenshot" "fullscreen screenshot is not accepted without PID-tied Morrow window proof"
      fi

      current_pids="$(pids_for_app_path | tr '\n' ' ' | sed 's/[[:space:]]*$//')"
      if [ -n "$current_pids" ]; then
        pids="$current_pids"
        record "PASS" "packaged Morrow process remains alive through screenshot capture" "temp app PID still present after screenshot"
      else
        record "FAIL" "packaged Morrow process remains alive through screenshot capture" "temp app PID exited before screenshot verification completed"
      fi

      set +e
      run_bounded_command "quit launched Morrow PID via osascript" osascript osascript - "$pids" <<'APPLESCRIPT'
on run argv
  set pidText to item 1 of argv
  set AppleScript's text item delimiters to " "
  set pidItems to text items of pidText
  tell application "System Events"
    repeat with pidItem in pidItems
      if pidItem as text is not "" then
        set targetPid to (pidItem as text) as integer
        set matchingProcesses to application processes whose unix id is targetPid
        if (count of matchingProcesses) > 0 then
          tell item 1 of matchingProcesses to quit
        end if
      end if
    end repeat
  end tell
end run
APPLESCRIPT
      quit_status=$?
      set -e
      [ "$quit_status" -eq 0 ] && record "PASS" "launched Morrow quit requested through osascript by PID" "exit 0 for pid(s): $pids" || record "FAIL" "launched Morrow quit requested through osascript by PID" "exit $quit_status for pid(s): $pids"

      if wait_for_no_app_pids; then
        record "PASS" "launched Morrow process exits after osascript quit" "no temp app PID remains"
      else
        record "PASS" "kill fallback required after osascript quit" "temp app PID still running; cleanup will kill exact launched PID"
      fi
    else
      record "FAIL" "packaged Morrow owns a PID-tied window with bounds before screenshot" "no launched process"
      record "FAIL" "screencapture creates screenshot artifact" "no launched process"
      record "FAIL" "screenshot artifact is nonblank by pixel probe" "no launched process"
      record "FAIL" "packaged Morrow window remains PID-tied with bounds after screenshot" "no launched process"
      record "FAIL" "packaged Morrow process remains alive through screenshot capture" "no launched process"
      record "FAIL" "launched Morrow quit requested through osascript by PID" "no launched process"
      record "FAIL" "kill fallback required after osascript quit" "no launched process"
    fi
  else
    process_text=""
    record "FAIL" "launch app Info.plist exists" "no Morrow.app"
    record "FAIL" "launch app LSRequiresCarbon is absent for windowed LaunchServices" "no Morrow.app"
    record "FAIL" "launch app CFBundleExecutable is declared" "no Morrow.app"
    record "FAIL" "launch app executable exists and is executable before open" "no Morrow.app"
    record "FAIL" "launch copy is non-quarantined before open" "no Morrow.app"
    record "FAIL" "open -n accepts packaged diagnostic Morrow.app" "no Morrow.app"
    record "FAIL" "packaged Morrow process starts from temp app path" "no Morrow.app"
    record "FAIL" "process identity records packaged app path" "no Morrow.app"
    record "FAIL" "packaged Morrow owns a PID-tied window with bounds before screenshot" "no Morrow.app"
    record "FAIL" "screencapture creates screenshot artifact" "no Morrow.app"
    record "FAIL" "screenshot artifact is nonblank by pixel probe" "no Morrow.app"
    record "FAIL" "packaged Morrow window remains PID-tied with bounds after screenshot" "no Morrow.app"
    record "FAIL" "packaged Morrow process remains alive through screenshot capture" "no Morrow.app"
    record "FAIL" "launched Morrow quit requested through osascript by PID" "no Morrow.app"
    record "FAIL" "launched Morrow process exits after osascript quit" "no Morrow.app"
  fi

  cleanup_temp
  deadline=$((SECONDS + 60))
  while [ "$SECONDS" -le "$deadline" ]; do
    exact_pids_after="$(pids_for_app_path | tr '\n' ' ' | sed 's/[[:space:]]*$//')"
    [ -z "$exact_pids_after" ] && break
    for pid in $exact_pids_after; do
      kill -KILL "$pid" 2>/dev/null || true
      cleanup_receipts+=("sent final KILL to launched Morrow PID $pid")
    done
    sleep 1
  done
  exact_pids_after="$(pids_for_app_path | tr '\n' ' ' | sed 's/[[:space:]]*$//')"
  if [ -z "$exact_pids_after" ]; then
    record "PASS" "no launched temp Morrow process remains after cleanup" "exact temp app process scan empty"
  else
    record "FAIL" "no launched temp Morrow process remains after cleanup" "remaining pid(s): $exact_pids_after"
  fi

  if [ -n "$temp_dir" ] && [ ! -e "$temp_dir" ]; then
    record "PASS" "temporary launch copy cleanup" "$temp_dir removed"
  else
    record "FAIL" "temporary launch copy cleanup" "${temp_dir:-missing} still exists"
  fi

  mount_scan="$(mounted_morrow_images || true)"
  if [ -z "$mount_scan" ]; then
    record "PASS" "no diagnostic DMG mount remains" "hdiutil scan contains no Morrow diagnostic mount"
  else
    record "FAIL" "no diagnostic DMG mount remains" "Morrow diagnostic mount remains"
  fi

  if printf '%s\n' "${checks[@]}" | grep -q '^FAIL'$'\t'; then
    result="FAIL"
  else
    result="PASS"
  fi
  write_evidence "$diagnostic_dir" "$evidence_path" "$result" "$primary_zip" "$screenshot_path" "$process_text" "$window_text" "$post_screenshot_window_text"
  rm -f "${tmp_files[@]}" 2>/dev/null || true
  [ "$result" = "PASS" ]
}

main "$@"
