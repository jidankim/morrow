#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="${TMPDIR:-/private/tmp}/morrow-reminders-real-qa"
BINARY_PATH="$BUILD_DIR/reminders_real_qa"

if [[ "${1:-}" != "" && "${1:-}" != "--request-access" ]]; then
  echo "status=failed"
  echo "reason=usage"
  echo "required_action=$SCRIPT_DIR/reminders_real_qa.sh [--request-access]"
  exit 64
fi

mkdir -p "$BUILD_DIR"

clang -fobjc-arc -Wall -Wextra -Werror \
  -framework Foundation \
  -framework EventKit \
  "$SCRIPT_DIR/reminders_real_qa.m" \
  "$SCRIPT_DIR/reminders_real_qa_support.m" \
  -o "$BINARY_PATH"

if [[ "${1:-}" == "--request-access" ]]; then
  "$BINARY_PATH" --request-access
else
  "$BINARY_PATH"
fi
