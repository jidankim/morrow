#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 1 ]; then
  printf 'tauri-dev-signed-runner: missing cargo command\n' >&2
  exit 64
fi

if [ "$1" != "run" ]; then
  exec cargo "$@"
fi

shift

build_args=()
app_args=()
profile="debug"
target_triple=""
after_separator=false
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(cd -- "$script_dir/.." && pwd -P)"

while [ "$#" -gt 0 ]; do
  if [ "$after_separator" = true ]; then
    app_args+=("$1")
    shift
    continue
  fi

  case "$1" in
    --)
      after_separator=true
      shift
      ;;
    --release)
      profile="release"
      build_args+=("$1")
      shift
      ;;
    --profile)
      build_args+=("$1" "$2")
      profile="$2"
      shift 2
      ;;
    --profile=*)
      build_args+=("$1")
      profile="${1#--profile=}"
      shift
      ;;
    --target)
      build_args+=("$1" "$2")
      target_triple="$2"
      shift 2
      ;;
    --target=*)
      build_args+=("$1")
      target_triple="${1#--target=}"
      shift
      ;;
    *)
      build_args+=("$1")
      shift
      ;;
  esac
done

cargo build -p morrow --bin morrow "${build_args[@]}"

target_dir="${CARGO_TARGET_DIR:-$repo_root/target}"
if [ -n "$target_triple" ]; then
  app_binary="$target_dir/$target_triple/$profile/morrow"
else
  app_binary="$target_dir/$profile/morrow"
fi

if [ "$(uname -s)" = "Darwin" ]; then
  codesign --force --sign - "$app_binary" >/dev/null
fi

if [ "${MORROW_TAURI_DEV_RUNNER_NO_EXEC:-}" = "1" ]; then
  printf '%s\n' "$app_binary"
  exit 0
fi

exec "$app_binary" "${app_args[@]}"
