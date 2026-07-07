#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: interruption-probe-child.sh <receipt> <label> <token>" >&2
  exit 125
fi

receipt="$1"
label="$2"
probe_token="$3"
sleep_seconds="${INTERRUPTION_PROBE_SLEEP_SECONDS:-10}"
sleep_pid=""

write_receipt() {
  local observed_signal="$1"
  local lifecycle="$2"
  mkdir -p "$(dirname -- "$receipt")"
  {
    echo "scenario: phase 6 cloud eval monitoring interruption probe"
    echo "label: $label"
    echo "probe_token: $probe_token"
    echo "child_pid: $$"
    echo "observed_signal: $observed_signal"
    echo "lifecycle: $lifecycle"
    echo "cleanup_receipt_written: yes"
    echo "result: CLEANUP_RECORDED"
  } > "$receipt"
}

on_interrupt() {
  local observed_signal="$1"
  if [[ -n "$sleep_pid" ]]; then
    kill "$sleep_pid" 2>/dev/null || true
    wait "$sleep_pid" 2>/dev/null || true
  fi
  write_receipt "$observed_signal" "terminated_by_timeout_wrapper"
  exit 143
}

trap 'on_interrupt TERM' TERM
trap 'on_interrupt INT' INT

write_receipt "none" "started"
while true; do
  sleep "$sleep_seconds" &
  sleep_pid="$!"
  wait "$sleep_pid"
done
