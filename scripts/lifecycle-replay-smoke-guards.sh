usage() {
  cat >&2 <<'EOF'
usage: run-lifecycle-replay-coverage-smoke.sh [--out-dir <path>] [--assert-canary-rejection]

Runs the Phase 3 lifecycle replay smoke with local storage/reconcile/diagnostics/native tests.
EOF
}

die() {
  echo "error: $*" >&2
  exit 64
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
}

quote_invocation() {
  local quoted="scripts/run-lifecycle-replay-coverage-smoke.sh"
  for arg in "$@"; do
    printf -v quoted '%s %q' "$quoted" "$arg"
  done
  printf '%s' "$quoted"
}

canonicalize_output_path() {
  local candidate="$1"
  local absolute
  local probe
  local suffix=""
  local base
  local probe_abs

  [[ -n "$candidate" ]] || die "--out-dir must not be empty"
  case "/$candidate/" in
    *"/../"*|*"/./"*) die "--out-dir must not contain . or .. path components: $candidate" ;;
  esac

  if [[ "$candidate" == /* ]]; then
    absolute="$candidate"
  else
    absolute="$repo_root_abs/$candidate"
  fi

  case "$absolute" in
    "/") die "--out-dir must be a dedicated evidence or tmp directory, not /" ;;
  esac

  while [[ "$absolute" != "/" && "$absolute" == */ ]]; do
    absolute="${absolute%/}"
  done

  probe="$absolute"
  while [[ ! -e "$probe" ]]; do
    base="$(basename -- "$probe")"
    [[ -n "$base" && "$base" != "/" && "$base" != "." ]] || die "invalid --out-dir: $candidate"
    suffix="/$base$suffix"
    probe="$(dirname -- "$probe")"
  done
  [[ -d "$probe" ]] || die "--out-dir parent is not a directory: $candidate"

  probe_abs="$(cd -- "$probe" && pwd -P)"
  printf '%s%s' "$probe_abs" "$suffix"
}

has_dedicated_tmp_leaf() {
  local candidate_abs="$1"
  local leaf

  leaf="$(basename -- "$candidate_abs")"
  case "$leaf" in
    morrow-lifecycle-replay-coverage-*-smoke|phase-3-lifecycle-replay-coverage-*-smoke) return 0 ;;
    *) return 1 ;;
  esac
}

is_dedicated_tmp_dir() {
  local candidate_abs="$1"
  local tmp_root
  local tmp_root_abs
  local tmp_child
  local seen_roots=" "

  for tmp_root in "${TMPDIR:-/tmp}" /tmp /private/tmp; do
    [[ -d "$tmp_root" ]] || continue
    tmp_root_abs="$(cd -- "$tmp_root" && pwd -P)" || return 1
    case "$seen_roots" in
      *" $tmp_root_abs "*) continue ;;
    esac
    seen_roots="$seen_roots$tmp_root_abs "
    case "$candidate_abs" in
      "$tmp_root_abs"/*)
        tmp_child="${candidate_abs#"$tmp_root_abs"/}"
        case "$tmp_child" in
          */*|"") continue ;;
        esac
        if has_dedicated_tmp_leaf "$candidate_abs"; then
          return 0
        fi
        ;;
    esac
  done

  return 1
}

require_empty_or_marked_tmp_dir() {
  local prepared_abs="$1"
  local entries

  [[ -d "$prepared_abs" ]] || return 0
  [[ -e "$prepared_abs/$OUT_DIR_MARKER" ]] && return 0

  shopt -s nullglob dotglob
  entries=("$prepared_abs"/*)
  shopt -u nullglob dotglob
  if (( ${#entries[@]} > 0 )); then
    die "--out-dir tmp cleanup requires an empty directory or existing smoke marker: $prepared_abs"
  fi
}

prepare_out_dir() {
  local requested="$1"
  local phase_evidence_abs
  local prepared_abs
  local phase_child

  phase_evidence_abs="$(canonicalize_output_path "$PHASE_EVIDENCE_DIR")" || exit $?
  prepared_abs="$(canonicalize_output_path "$requested")" || exit $?

  case "$prepared_abs" in
    /|"$repo_root_abs"|"$phase_evidence_abs")
      die "--out-dir must be a dedicated evidence or tmp directory, not $prepared_abs"
      ;;
  esac

  case "$prepared_abs" in
    "$phase_evidence_abs"/*)
      phase_child="${prepared_abs#"$phase_evidence_abs"/}"
      case "$phase_child" in
        "") die "--out-dir must be a dedicated smoke directory under $PHASE_EVIDENCE_DIR: $requested" ;;
        *-smoke) ;;
        *) die "--out-dir must be a dedicated smoke directory under $PHASE_EVIDENCE_DIR: $requested" ;;
      esac
      ;;
    *)
      if ! is_dedicated_tmp_dir "$prepared_abs"; then
        die "--out-dir cleanup is only allowed under $PHASE_EVIDENCE_DIR or a dedicated TMPDIR/private tmp smoke directory: $requested"
      fi
      require_empty_or_marked_tmp_dir "$prepared_abs"
      ;;
  esac

  mkdir -p "$prepared_abs"
  printf 'phase-3 lifecycle replay coverage smoke out-dir\n' > "$prepared_abs/$OUT_DIR_MARKER"
  cd -- "$prepared_abs" && pwd -P
}

safe_remove_path() {
  local target="$1"
  [[ -n "$target" ]] || die "refusing to remove an empty path"
  case "$target" in
    "$out_dir_abs"/*) rm -rf "$target" ;;
    *) die "refusing to remove path outside --out-dir: $target" ;;
  esac
}

require_file() {
  local path="$1"
  [[ -s "$path" ]] || die "expected non-empty artifact: $path"
}

reject_literal_in_file() {
  local literal="$1"
  local path="$2"
  if grep -Fq "$literal" "$path"; then
    die "forbidden literal leaked into $path"
  fi
}
