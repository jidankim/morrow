#!/usr/bin/env bash
set -u

usage="Usage: scripts/codex-setup-real-surface-qa.sh .omo/evidence/native-codex-login-flow/real-surface"

if [ "$#" -ne 1 ]; then
  printf '%s\n' "$usage" >&2
  exit 2
fi

evidence_dir="$1"
summary_path="$evidence_dir/summary.txt"
version_path="$evidence_dir/codex-version.txt"
status_path="$evidence_dir/login-status-classification.txt"
cleanup_path="$evidence_dir/cleanup-receipt.txt"
run_started="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
tmp_dir="$(mktemp -d /tmp/morrow-codex-real-surface-XXXXXX)"
cleanup_status="pending"
bounded_timeout_seconds="${CODEX_SETUP_QA_TIMEOUT_SECONDS:-20}"
bounded_grace_seconds="${CODEX_SETUP_QA_TERM_GRACE_SECONDS:-3}"

mkdir -p "$evidence_dir"

case "$bounded_timeout_seconds" in
  ''|*[!0-9]*) bounded_timeout_seconds=20 ;;
esac

case "$bounded_grace_seconds" in
  ''|*[!0-9]*) bounded_grace_seconds=3 ;;
esac

contains_sensitive_text() {
  LC_ALL=C grep -Eiq '(codex[_-]?access[_-]?token|auth[.]json|[~]/[.]codex|sk-[A-Za-z0-9]{8,}|[A-Za-z0-9_=-]{32,})'
}

contains_negative_login_status() {
  LC_ALL=C grep -Eiq '(not[[:space:]-]+authenticated|unauthenticated|not[[:space:]-]+logged[[:space:]-]+in|logged[[:space:]-]+out|signed[[:space:]-]+out|sign[[:space:]-]+in[[:space:]-]+required|login[[:space:]-]+required|authentication[[:space:]-]+required|no[[:space:]-]+active[[:space:]-]+session)'
}

contains_falsy_ready_status() {
  LC_ALL=C grep -Eiq "(^|[^[:alnum:]_])['\"]?[[:space:]]*((authenticated)|((logged|signed)[[:space:]_-]*in)|(login[[:space:]_-]+status))[[:space:]]*['\"]?[[:space:]]*[:=][[:space:]]*((['\"]?[[:space:]]*(false|no|none|null|nil|0|unknown)[[:space:]]*['\"]?)([^[:alnum:]_]|\$)|(['\"]?[[:space:]]*['\"]?)[[:space:]]*([,;)}\]]|\$)|(\[\]|\{\}|\[|\{))"
}

contains_ready_login_status() {
  LC_ALL=C grep -Eiq "((authenticated|logged[[:space:]_-]*in|signed[[:space:]_-]*in)[[:space:]_-]*(with|using|via|to|for)[[:space:]_-]*chatgpt)|(chatgpt[[:space:]_-]*(ready|authenticated|logged[[:space:]_-]*in|signed[[:space:]_-]*in))|((chatgpt|provider[[:space:]_-]*chatgpt)[[:space:]]*(auth|login|status|ready)?[[:space:]]*[:=][[:space:]]*(true|yes|1|ready|authenticated|logged[[:space:]_-]*in|signed[[:space:]_-]*in))|(chatgpt.*['\"]?[[:space:]]*((authenticated)|((logged|signed)[[:space:]_-]*in)|(login[[:space:]_-]+status))[[:space:]]*['\"]?[[:space:]]*[:=][[:space:]]*['\"]?[[:space:]]*(true|yes|1|ready|authenticated|logged[[:space:]_-]*in|signed[[:space:]_-]*in))|((['\"]?[[:space:]]*((authenticated)|((logged|signed)[[:space:]_-]*in)|(login[[:space:]_-]+status))[[:space:]]*['\"]?[[:space:]]*[:=][[:space:]]*['\"]?[[:space:]]*(true|yes|1|ready|authenticated|logged[[:space:]_-]*in|signed[[:space:]_-]*in)).*chatgpt)"
}

run_bounded() {
  output_path="$1"
  shift
  "$@" > "$output_path" 2>&1 &
  command_pid="$!"
  elapsed=0
  while kill -0 "$command_pid" 2>/dev/null; do
    if [ "$elapsed" -ge "$bounded_timeout_seconds" ]; then
      kill -TERM "$command_pid" 2>/dev/null || true
      grace_elapsed=0
      while kill -0 "$command_pid" 2>/dev/null && [ "$grace_elapsed" -lt "$bounded_grace_seconds" ]; do
        sleep 1
        grace_elapsed=$((grace_elapsed + 1))
      done
      if kill -0 "$command_pid" 2>/dev/null; then
        kill -KILL "$command_pid" 2>/dev/null || true
      fi
      wait "$command_pid" 2>/dev/null || true
      return 124
    fi
    sleep 1
    elapsed=$((elapsed + 1))
  done
  wait "$command_pid"
}

write_cleanup() {
  if [ -d "$tmp_dir" ]; then
    rm -rf "$tmp_dir"
    cleanup_status="temp dir removed"
  fi
  {
    printf 'Scenario: real Codex setup surface cleanup\n'
    printf 'Invocation: scripts/codex-setup-real-surface-qa.sh %s\n' "$evidence_dir"
    printf 'Binary observable: no login process or server started by this script\n'
    printf 'Result: PASS cleanup complete\n'
    printf 'Temp dir: %s\n' "$cleanup_status"
    printf 'Recorded at: %s\n' "$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  } > "$cleanup_path"
}

record_blocked() {
  reason="$1"
  {
    printf 'Scenario: sanitized local Codex setup surface\n'
    printf 'Invocation: scripts/codex-setup-real-surface-qa.sh %s\n' "$evidence_dir"
    printf 'Binary observable: %s\n' "$reason"
    printf 'Result: %s\n' "$reason"
    printf 'Run started: %s\n' "$run_started"
    printf 'Artifacts: %s, %s, %s\n' "$summary_path" "$status_path" "$cleanup_path"
  } > "$summary_path"
  printf '%s\n' "$reason" > "$status_path"
  write_cleanup
  printf '%s\n' "$reason"
}

codex_path="$(command -v codex || true)"
if [ -z "$codex_path" ]; then
  record_blocked "BLOCKED: codex missing"
  exit 0
fi

version_output_path="$tmp_dir/version.out"
run_bounded "$version_output_path" codex --version
version_status="$?"
version_output="$(cat "$version_output_path")"
if printf '%s\n' "$version_output" | contains_sensitive_text; then
  {
    printf 'Scenario: codex version sanitization\n'
    printf 'Invocation: codex --version\n'
    printf 'Binary observable: token-like output rejected\n'
    printf 'Result: FAIL sensitive version output\n'
  } > "$summary_path"
  write_cleanup
  printf 'FAIL sensitive version output\n' >&2
  exit 1
fi

{
  printf 'Scenario: codex version probe\n'
  printf 'Invocation: codex --version\n'
  printf 'Binary observable: process exit status %s\n' "$version_status"
  printf 'Result: %s\n' "$version_output"
} > "$version_path"

status_output_path="$tmp_dir/login-status.out"
run_bounded "$status_output_path" codex login status
status_exit="$?"
status_output="$(cat "$status_output_path")"
if printf '%s\n' "$status_output" | contains_sensitive_text; then
  {
    printf 'Scenario: sanitized codex login status classification\n'
    printf 'Invocation: codex login status\n'
    printf 'Binary observable: token-like output rejected\n'
    printf 'Result: FAIL sensitive login status output\n'
  } > "$status_path"
  write_cleanup
  printf 'FAIL sensitive login status output\n' >&2
  exit 1
elif printf '%s\n' "$status_output" | contains_falsy_ready_status; then
  status_classification="BLOCKED: user login required"
elif printf '%s\n' "$status_output" | contains_negative_login_status; then
  status_classification="BLOCKED: user login required"
elif [ "$status_exit" -eq 0 ] && printf '%s\n' "$status_output" | contains_ready_login_status; then
  status_classification="PASS ready"
else
  status_classification="BLOCKED: user login required"
fi

{
  printf 'Scenario: sanitized codex login status classification\n'
  printf 'Invocation: codex login status\n'
  printf 'Binary observable: process exit status %s, raw output not recorded\n' "$status_exit"
  printf 'Result: %s\n' "$status_classification"
} > "$status_path"

{
  printf 'Scenario: sanitized local Codex setup surface\n'
  printf 'Invocation: scripts/codex-setup-real-surface-qa.sh %s\n' "$evidence_dir"
  printf 'Binary observable: codex path present, version sanitized, login status classified\n'
  printf 'Result: %s\n' "$status_classification"
  printf 'Run started: %s\n' "$run_started"
  printf 'Artifacts: %s, %s, %s, %s\n' "$summary_path" "$version_path" "$status_path" "$cleanup_path"
} > "$summary_path"

write_cleanup
printf '%s\n' "$status_classification"
exit 0
