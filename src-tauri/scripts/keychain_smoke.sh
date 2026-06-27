#!/usr/bin/env bash
set -u

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LOG_PATH="${1:-"$ROOT_DIR/.omo/evidence/task-2-keychain-verification-repair-smoke.log"}"
TARGET_DIR="${MORROW_KEYCHAIN_SMOKE_TARGET_DIR:-/private/tmp/morrow-task-2-keychain-smoke}"
SDKROOT_VALUE="${SDKROOT:-/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk}"
LINKER_VALUE="${MORROW_KEYCHAIN_SMOKE_LINKER:-/Library/Developer/CommandLineTools/usr/bin/cc}"
SERVICE="com.morrow.desktop.token"
ACCOUNT="morrow-owned-token"

mkdir -p "$(dirname "$LOG_PATH")"
: > "$LOG_PATH"

log() {
  printf '%s\n' "$*" >> "$LOG_PATH"
}

cleanup() {
  if [ -x "$TARGET_DIR/debug/examples/keychain_smoke" ]; then
    MORROW_KEYCHAIN_SMOKE_TOKEN="cleanup-redacted" "$TARGET_DIR/debug/examples/keychain_smoke" cleanup >> "$LOG_PATH" 2>&1 || true
  fi
  security delete-generic-password -s "$SERVICE" -a "$ACCOUNT" >> "$LOG_PATH" 2>&1 || true
  rm -rf "$TARGET_DIR"
}

trap cleanup EXIT

TOKEN="morrow-smoke-$(uuidgen)-$(date +%s)"
TOKEN_LEN=$(printf '%s' "$TOKEN" | wc -c | tr -d '[:space:]')
TOKEN_SHA256=$(printf '%s' "$TOKEN" | shasum -a 256 | awk '{print $1}')

log "scenario=real macOS Keychain smoke through production NativeBridgeState"
log "date=$(date -Iseconds)"
log "workspace=$ROOT_DIR"
log "target_dir=$TARGET_DIR"
log "service=$SERVICE"
log "account=$ACCOUNT"
log "token_len=$TOKEN_LEN"
log "token_sha256=$TOKEN_SHA256"
log "token_value_logged=false"
log "sdkroot=$SDKROOT_VALUE"
log "rustflags=-C linker=$LINKER_VALUE"

rm -rf "$TARGET_DIR"
(
  cd "$ROOT_DIR"
  CARGO_TARGET_DIR="$TARGET_DIR" \
    SDKROOT="$SDKROOT_VALUE" \
    RUSTFLAGS="-C linker=$LINKER_VALUE" \
    cargo build --manifest-path src-tauri/Cargo.toml --example keychain_smoke
) >> "$LOG_PATH" 2>&1
BUILD_EXIT=$?
log "cargo_build_exit=$BUILD_EXIT"
if [ "$BUILD_EXIT" -ne 0 ]; then
  exit "$BUILD_EXIT"
fi

BIN="$TARGET_DIR/debug/examples/keychain_smoke"
log "binary=$BIN"
file "$BIN" >> "$LOG_PATH" 2>&1 || true
codesign -dv --entitlements :- "$BIN" >> "$LOG_PATH" 2>&1 || true
codesign --force --sign - "$BIN" >> "$LOG_PATH" 2>&1
SIGN_EXIT=$?
log "codesign_exit=$SIGN_EXIT"
if [ "$SIGN_EXIT" -ne 0 ]; then
  exit "$SIGN_EXIT"
fi
codesign -dv --entitlements :- "$BIN" >> "$LOG_PATH" 2>&1 || true

MORROW_KEYCHAIN_SMOKE_TOKEN="$TOKEN" "$BIN" >> "$LOG_PATH" 2>&1
SMOKE_EXIT=$?
log "smoke_exit=$SMOKE_EXIT"

security find-generic-password -s "$SERVICE" -a "$ACCOUNT" >/dev/null 2>> "$LOG_PATH"
FIND_AFTER_CLEANUP_EXIT=$?
log "security_find_after_cleanup_exit=$FIND_AFTER_CLEANUP_EXIT"

if [ "$SMOKE_EXIT" -ne 0 ]; then
  exit "$SMOKE_EXIT"
fi
if [ "$FIND_AFTER_CLEANUP_EXIT" -ne 44 ]; then
  log "cleanup_verification=unexpected_find_exit"
  exit 1
fi

log "cleanup_verification=not_found"
log "keychain_smoke_result=pass"
