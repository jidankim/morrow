#!/usr/bin/env bash
set -euo pipefail

is_full_xcode_developer_dir() {
  local developer_dir="$1"
  if [ -z "$developer_dir" ] || [ ! -x "$developer_dir/usr/bin/xcodebuild" ]; then
    return 1
  fi

  local xcode_version
  xcode_version="$("$developer_dir/usr/bin/xcodebuild" -version 2>/dev/null || true)"
  [[ "$xcode_version" == Xcode\ * ]]
}

select_xcode_developer_dir() {
  if [ -n "${DEVELOPER_DIR:-}" ] && is_full_xcode_developer_dir "$DEVELOPER_DIR"; then
    return
  fi

  local selected_dir
  selected_dir="$(xcode-select -p 2>/dev/null || true)"
  if is_full_xcode_developer_dir "$selected_dir"; then
    export DEVELOPER_DIR="$selected_dir"
    return
  fi

  local candidate
  shopt -s nullglob
  local candidates=(
    /Applications/Xcode.app/Contents/Developer
    "$HOME"/Applications/Xcode.app/Contents/Developer
    /Applications/Xcode-*.app/Contents/Developer
    "$HOME"/Applications/Xcode-*.app/Contents/Developer
  )
  shopt -u nullglob

  for candidate in "${candidates[@]}"; do
    if is_full_xcode_developer_dir "$candidate"; then
      export DEVELOPER_DIR="$candidate"
      return
    fi
  done
}

is_simulator_build() {
  [ "${1:-}" = "build" ] || return 1

  local previous_arg=""
  local arg
  for arg in "$@"; do
    if [ "$previous_arg" = "--target" ] && [ "$arg" = "aarch64-sim" ]; then
      return 0
    fi
    if [ "$arg" = "--target=aarch64-sim" ]; then
      return 0
    fi
    previous_arg="$arg"
  done

  return 1
}

clean_simulator_build_output() {
  local project_root
  project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"

  local apple_build_dir="$project_root/src-tauri/gen/apple/build"
  rm -rf -- "$apple_build_dir/arm64-sim" "$apple_build_dir/morrow_iOS.xcarchive"
}

if is_simulator_build "$@"; then
  clean_simulator_build_output
fi

select_xcode_developer_dir
if [ -n "${DEVELOPER_DIR:-}" ] && [ -x "$DEVELOPER_DIR/usr/bin/xcodebuild" ]; then
  export PATH="$DEVELOPER_DIR/usr/bin:$PATH"

  clang_path="$(xcrun --find clang 2>/dev/null || true)"
  clangxx_path="$(xcrun --find clang++ 2>/dev/null || true)"
  ar_path="$(xcrun --find ar 2>/dev/null || true)"
  ranlib_path="$(xcrun --find ranlib 2>/dev/null || true)"

  if [ -x "$clang_path" ]; then
    toolchain_bin="$(dirname "$clang_path")"
    export PATH="$toolchain_bin:$PATH"
    export CC="${CC:-$clang_path}"
    export CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER="${CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER:-$clang_path}"
    export CARGO_TARGET_AARCH64_APPLE_IOS_LINKER="${CARGO_TARGET_AARCH64_APPLE_IOS_LINKER:-$clang_path}"
    export CARGO_TARGET_AARCH64_APPLE_IOS_SIM_LINKER="${CARGO_TARGET_AARCH64_APPLE_IOS_SIM_LINKER:-$clang_path}"
    export CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER="${CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER:-$clang_path}"
    export CARGO_TARGET_X86_64_APPLE_IOS_LINKER="${CARGO_TARGET_X86_64_APPLE_IOS_LINKER:-$clang_path}"
    export CARGO_TARGET_X86_64_APPLE_IOS_SIM_LINKER="${CARGO_TARGET_X86_64_APPLE_IOS_SIM_LINKER:-$clang_path}"
  fi
  if [ -x "$clangxx_path" ]; then
    export CXX="${CXX:-$clangxx_path}"
  fi
  if [ -x "$ar_path" ]; then
    export AR="${AR:-$ar_path}"
  fi
  if [ -x "$ranlib_path" ]; then
    export RANLIB="${RANLIB:-$ranlib_path}"
  fi
fi

if command -v rustup >/dev/null 2>&1; then
  exec npx --no-install tauri ios "$@"
fi

if command -v nix >/dev/null 2>&1; then
  exec nix shell nixpkgs#rustup --command npx --no-install tauri ios "$@"
fi

cat >&2 <<'EOF'
Morrow Tauri iOS commands require rustup.

Install rustup, or install Nix so this script can run Tauri with:
  nix shell nixpkgs#rustup
EOF
exit 127
