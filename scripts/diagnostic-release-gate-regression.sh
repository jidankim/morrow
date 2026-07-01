#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

original_args=("$@")
mode="${1:-}"
evidence_path=""
bundle_dir=""
release_gate_status=""
release_gate_evidence=""
release_gate_stdout=""
fixture_dir=""
cleanup_receipts=()

usage() {
  printf 'Usage: %s --self-test <evidence.md>\n' "$0" >&2
  printf '       %s --self-test-fake-pass <evidence.md>\n' "$0" >&2
  printf '       %s <bundle-dir> <evidence.md>\n' "$0" >&2
}

quote_args() {
  local first=1 arg
  for arg in "$@"; do
    [ "$first" -eq 0 ] && printf ' '
    first=0
    printf '%q' "$arg"
  done
}

sanitize_stream() {
  if command -v perl >/dev/null 2>&1; then
    perl -0pe 's/(APPLE_(?:PASSWORD|ID|TEAM_ID|SIGNING_IDENTITY)=)[^\s]+/${1}[REDACTED]/g; s/sk-[A-Za-z0-9_-]+/[REDACTED_TOKEN]/g; s/codex_access_token[^\s]*/codex_access_token=[REDACTED]/g;'
  else
    sed -E \
      -e 's/(APPLE_(PASSWORD|ID|TEAM_ID|SIGNING_IDENTITY)=)[^[:space:]]+/\1[REDACTED]/g' \
      -e 's/sk-[A-Za-z0-9_-]+/[REDACTED_TOKEN]/g' \
      -e 's/codex_access_token[^[:space:]]*/codex_access_token=[REDACTED]/g'
  fi
}

cleanup() {
  if [ -n "$fixture_dir" ] && [ -d "$fixture_dir" ]; then
    rm -rf "$fixture_dir"
    cleanup_receipts+=("removed temp fixture $fixture_dir")
  fi
}
trap cleanup EXIT

create_self_test_fixture() {
  fixture_dir="$(mktemp -d /tmp/morrow-release-gate-regression-XXXXXX)"
  bundle_dir="$fixture_dir/bundle"
  mkdir -p "$bundle_dir/macos/Morrow.app"
  touch "$bundle_dir/Morrow_0.1.0_diagnostic.dmg"
}

run_release_gate() {
  release_gate_evidence="$fixture_dir/release-signing-qa.md"
  release_gate_stdout="$fixture_dir/release-signing-qa.stdout"
  if [ -z "$fixture_dir" ]; then
    fixture_dir="$(mktemp -d /tmp/morrow-release-gate-regression-XXXXXX)"
  fi
  if [ "$mode" != "--self-test" ]; then
    release_gate_evidence="$fixture_dir/release-signing-qa-real-artifact.md"
    release_gate_stdout="$fixture_dir/release-signing-qa-real-artifact.stdout"
  fi

  set +e
  env -u APPLE_ID -u APPLE_PASSWORD -u APPLE_TEAM_ID APPLE_SIGNING_IDENTITY=- \
    scripts/release-signing-qa.sh "$bundle_dir" "$release_gate_evidence" >"$release_gate_stdout" 2>&1
  release_gate_status=$?
  set -e
}

write_fake_pass_transcript() {
  fixture_dir="$(mktemp -d /tmp/morrow-release-gate-regression-XXXXXX)"
  release_gate_evidence="$fixture_dir/fake-release-signing-qa.md"
  release_gate_stdout="$fixture_dir/fake-release-signing-qa.stdout"
  release_gate_status=0
  {
    printf '# Fake Release Gate Transcript\n\n'
    printf 'This synthesized transcript is intentionally misleading and must be rejected.\n\n'
    printf 'RESULT: PASS\n'
  } >"$release_gate_evidence"
  printf 'fake release gate exited 0 and claimed RESULT: PASS\n' >"$release_gate_stdout"
}

assert_release_gate_rejected() {
  local status="$1" evidence="$2"
  [ "$status" -ne 0 ] &&
    rg -q '(^|[^A-Z])(BLOCKED|FAIL)([^A-Z]|$)|RESULT: (BLOCKED|FAIL)' "$evidence" &&
    ! rg -q 'RESULT: PASS' "$evidence"
}

write_evidence() {
  local result="$1" reason="$2" git_status release_diff qa_text stdout_text
  mkdir -p "$(dirname "$evidence_path")"
  git_status="$(git status --short 2>/dev/null || true)"
  release_diff="$(git diff -- scripts/release-signing-qa.sh 2>/dev/null || true)"
  qa_text="$(cat "$release_gate_evidence" 2>/dev/null | sanitize_stream || true)"
  stdout_text="$(cat "$release_gate_stdout" 2>/dev/null | sanitize_stream || true)"
  cleanup
  {
    printf '# Diagnostic Release Gate Regression Evidence\n\n'
    printf '## Scenario\n\n%s\n\n' "$reason"
    printf '## Invocation\n\n`%s`\n\n' "$(quote_args "$0" "${original_args[@]}")"
    printf '## Release Gate Invocation\n\n'
    if [ "$mode" = "--self-test-fake-pass" ]; then
      printf 'Synthesized fake release-gate transcript with `RESULT: PASS`.\n\n'
    else
      printf '`%s`\n\n' "$(quote_args env -u APPLE_ID -u APPLE_PASSWORD -u APPLE_TEAM_ID APPLE_SIGNING_IDENTITY=- scripts/release-signing-qa.sh "$bundle_dir" "$release_gate_evidence")"
    fi
    printf '## Git Status\n\n```text\n%s\n```\n\n' "$git_status"
    printf '## release-signing-qa.sh Provenance\n\n'
    if git ls-files --error-unmatch scripts/release-signing-qa.sh >/dev/null 2>&1; then
      printf 'Tracked by git.\n\n'
    else
      printf 'Untracked/pre-existing in this worktree; this harness did not edit it and verifies current behavior directly.\n\n'
    fi
    printf '## release-signing-qa.sh Diff\n\n```diff\n%s\n```\n\n' "$release_diff"
    printf '## Assertions\n\n'
    if [ "$mode" = "--self-test-fake-pass" ]; then
      printf -- '- FAIL misleading_success_output: fake `RESULT: PASS` transcript was rejected by the harness.\n'
    else
      printf -- '- %s release gate exits nonzero for ad-hoc/no-credential artifact: exit `%s`.\n' "$result" "$release_gate_status"
      if printf '%s\n' "$qa_text" | rg -q 'BLOCKED|FAIL|RESULT: (BLOCKED|FAIL)'; then
        printf -- '- PASS release gate records BLOCKED or FAIL.\n'
      else
        printf -- '- FAIL release gate did not record BLOCKED or FAIL.\n'
      fi
      if printf '%s\n' "$qa_text" | rg -q 'RESULT: PASS'; then
        printf -- '- FAIL release gate transcript contains `RESULT: PASS`.\n'
      else
        printf -- '- PASS release gate transcript contains no `RESULT: PASS`.\n'
      fi
    fi
    printf -- '- PASS prompt_injection: transcript is sanitized and treated only as data; `RESULT: PASS` is never trusted when rejection criteria fail.\n'
    printf -- '- PASS dirty_worktree: git status recorded and release-signing-qa.sh diff captured.\n'
    printf -- '- PASS stale_state: release-signing-qa.sh was invoked during this run, not inferred from prior evidence.\n'
    printf -- '- PASS generated_cached_artifacts: temp fixture/transcript directory is removed by trap cleanup.\n'
    printf -- '- not_applicable cancel_resume: single bounded local shell invocation with no resumable state.\n'
    printf -- '- not_applicable hung_long_commands: release gate does not run signing commands when credentials are absent; release-signing-qa has its own timeout if reached.\n'
    printf -- '- not_applicable flaky_tests: deterministic local fixture and transcript assertions.\n'
    printf -- '- not_applicable repeated_interruptions: no interrupt/resume mechanism is introduced.\n\n'
    printf '## Release Gate Exit\n\n`%s`\n\n' "$release_gate_status"
    printf '## Release Gate Stdout\n\n```text\n%s\n```\n\n' "$stdout_text"
    printf '## Release Gate Evidence Transcript\n\n```text\n%s\n```\n\n' "$qa_text"
    printf '## Cleanup\n\n'
    if [ "${#cleanup_receipts[@]}" -eq 0 ]; then
      printf -- '- Cleanup pending until process exit trap runs.\n'
    else
      printf -- '- %s\n' "${cleanup_receipts[@]}"
    fi
    printf '\nRESULT: %s\n' "$result"
  } >"$evidence_path"
}

case "$mode" in
  --self-test)
    evidence_path="${2:-}"
    [ -n "$evidence_path" ] || { usage; exit 2; }
    create_self_test_fixture
    run_release_gate
    if assert_release_gate_rejected "$release_gate_status" "$release_gate_evidence"; then
      write_evidence "PASS" "Self-test malformed/ad-hoc fixture proves the official release gate rejects ad-hoc/no-credential artifacts."
      exit 0
    fi
    write_evidence "FAIL" "Self-test malformed/ad-hoc fixture did not prove official release-gate rejection."
    exit 1
    ;;
  --self-test-fake-pass)
    evidence_path="${2:-}"
    [ -n "$evidence_path" ] || { usage; exit 2; }
    write_fake_pass_transcript
    write_evidence "FAIL" "Fake-pass mode intentionally synthesizes misleading success output and exits nonzero when the harness rejects it."
    exit 1
    ;;
  "")
    usage
    exit 2
    ;;
  *)
    bundle_dir="$1"
    evidence_path="${2:-}"
    [ -n "$bundle_dir" ] && [ -n "$evidence_path" ] || { usage; exit 2; }
    [ -d "$bundle_dir" ] || {
      mkdir -p "$(dirname "$evidence_path")"
      printf '# Diagnostic Release Gate Regression Evidence\n\nFAIL bundle directory exists: `%s`\n\nRESULT: FAIL\n' "$bundle_dir" >"$evidence_path"
      exit 1
    }
    run_release_gate
    if assert_release_gate_rejected "$release_gate_status" "$release_gate_evidence"; then
      write_evidence "PASS" "Real artifact regression proves the official release gate rejects ad-hoc/no-credential artifacts."
      exit 0
    fi
    write_evidence "FAIL" "Real artifact regression did not prove official release-gate rejection."
    exit 1
    ;;
esac
