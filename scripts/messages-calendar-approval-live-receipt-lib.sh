#!/usr/bin/env bash

field() {
  local key="$1"
  grep -E "^${key}=" "$summary_file" | tail -n 1 | cut -d= -f2-
}

write_cleanup() {
  local status="$1"
  local proof="$2"
  printf 'cleanup_status=%s\ncleanup_proof=%s\nraw_event_ids_dumped=false\n' "$status" "$proof" > "$cleanup_file"
}

write_privacy() {
  local status="$1"
  printf 'privacy_scan=%s\nprovider_payload_dumped=false\ncodex_stdout_dumped=false\ncodex_stderr_dumped=false\nmessage_body_dumped=false\nraw_handles_dumped=false\n' "$status" > "$privacy_file"
}

is_fixture_source() {
  [[ -n "${fixture_case:-}" ]]
}

full_live_claim_allowed() {
  local overall="$1"
  local before_mutation="$2"
  local cleanup_status="$3"
  local privacy_status="$4"
  [[ "$overall" == "PASS" ]] || return 1
  ! is_fixture_source || return 1
  [[ "${controlled_live_proof_marker_present:-false}" == "true" ]] || return 1
  [[ "${controlled_proposal_created:-false}" == "true" ]] || return 1
  [[ "$before_mutation" == "false" ]] || return 1
  [[ "$cleanup_status" == "PASS" && "$privacy_status" == "PASS" ]] || return 1
  for status in \
    "${provider_status:-BLOCKED}" \
    "${messages_status:-BLOCKED}" \
    "${selected_chat_status:-BLOCKED}" \
    "${calendar_status:-BLOCKED}" \
    "${reminders_status:-BLOCKED}" \
    "${proposal_status:-BLOCKED}" \
    "${readback_status:-BLOCKED}" \
    "${qa_action_status:-BLOCKED}" \
    "${reconcile_status:-BLOCKED}" \
    "${idempotency_status:-BLOCKED}"; do
    [[ "$status" == "PASS" ]] || return 1
  done
}

write_summary() {
  local overall="$1"
  local blockers="$2"
  local before_mutation="$3"
  local cleanup_status="$4"
  local privacy_status="$5"
  local full_claim="false"
  full_live_claim_allowed "$overall" "$before_mutation" "$cleanup_status" "$privacy_status" && full_claim="true"
  {
    printf 'schema=phase5_messages_calendar_approval_live_receipt_v1\n'
    printf 'run_id=phase5-live-receipt-%s-%s\n' "$(date -u '+%Y%m%dT%H%M%SZ')" "$$"
    printf 'invocation=scripts/run-messages-calendar-approval-live-receipt.sh --out-dir %s\n' "$out_dir"
    printf 'provider_source_of_truth=Codex CLI session\n'
    printf 'preflight_report=%s/preflight/preflight-report.json\n' "$phase_root"
    printf 'fixture_source=%s\n' "$(is_fixture_source && printf true || printf false)"
    printf 'controlled_live_proof_marker_present=%s\n' "${controlled_live_proof_marker_present:-false}"
    printf 'overall_status=%s\n' "$overall"
    printf 'provider_readiness_status=%s\n' "${provider_status:-BLOCKED}"
    printf 'messages_access_status=%s\n' "${messages_status:-BLOCKED}"
    printf 'selected_test_chat_status=%s\n' "${selected_chat_status:-BLOCKED}"
    printf 'calendar_access_status=%s\n' "${calendar_status:-BLOCKED}"
    printf 'reminders_access_status=%s\n' "${reminders_status:-BLOCKED}"
    printf 'proposal_creation_status=%s\n' "${proposal_status:-BLOCKED}"
    printf 'readback_status=%s\n' "${readback_status:-BLOCKED}"
    printf 'qa_action_observation_status=%s\n' "${qa_action_status:-BLOCKED}"
    printf 'reconcile_status=%s\n' "${reconcile_status:-BLOCKED}"
    printf 'idempotency_status=%s\n' "${idempotency_status:-BLOCKED}"
    printf 'privacy_inspection_status=%s\n' "$privacy_status"
    printf 'cleanup_status=%s\n' "$cleanup_status"
    printf 'controlled_proposal_created=%s\n' "${controlled_proposal_created:-false}"
    printf 'blocked_before_mutation=%s\n' "$before_mutation"
    printf 'blocker_reasons=%s\n' "$blockers"
    printf 'full_live_claim_allowed=%s\n' "$full_claim"
  } > "$summary_file"
}

ensure_surface_receipts() {
  [[ -s "$provider_file" ]] || printf 'provider_readiness_status=%s\nreason=not_reached\n' "${provider_status:-BLOCKED}" > "$provider_file"
  [[ -s "$messages_file" ]] || printf 'messages_access_status=%s\nreason=not_reached\n' "${messages_status:-BLOCKED}" > "$messages_file"
  [[ -s "$calendar_file" ]] || printf 'calendar_access_status=%s\nreminders_access_status=%s\nreason=not_reached_before_mutation\n' "${calendar_status:-BLOCKED}" "${reminders_status:-BLOCKED}" > "$calendar_file"
}

validate_receipt() {
  [[ -s "$summary_file" ]] || { printf 'FAIL validator: missing summary\n' >&2; return 1; }
  if find "$repo_root/$out_dir" -maxdepth 1 -type f \
    ! -name source-diff.patch \
    ! -name done-claim.md \
    ! -name failure-path-matrix.txt \
    ! -name positive-path-matrix.txt \
    ! -name privacy-forbidden-scan.txt \
    ! -name fixture-raw-payload-scan.txt \
    ! -name git-status-after-todo6.txt \
    ! -name evidence-file-sizes.txt \
    ! -name pure-loc-by-file.txt \
    ! -name syntax-main.txt \
    ! -name syntax-lib.txt \
    ! -name git-diff-check.txt \
    -exec grep -E 'raw-provider-request-payload|raw-provider-response-body|codex_access_token|[p]rovider_payload_json|fixture_forbidden_payload_marker|\{[[:space:]]*"model"[[:space:]]*:[[:space:]]*"' {} + >/dev/null 2>&1; then
    printf 'FAIL validator: provider payload or secret-like evidence was printed\n' >&2
    return 1
  fi
  if grep -q '=FAIL$' "$summary_file"; then
    printf 'FAIL validator: receipt contains FAIL status\n' >&2
    return 1
  fi
  [[ -s "$cleanup_file" ]] || { printf 'FAIL validator: missing cleanup receipt\n' >&2; return 1; }
  [[ -s "$privacy_file" ]] || { printf 'FAIL validator: missing privacy scan\n' >&2; return 1; }
  grep -q '^privacy_scan=PASS$' "$privacy_file" || { printf 'FAIL validator: privacy scan did not pass\n' >&2; return 1; }
  local overall cleanup full_claim
  overall="$(field overall_status)"
  cleanup="$(field cleanup_status)"
  full_claim="$(field full_live_claim_allowed)"
  case "$overall" in
    PASS)
      [[ "$cleanup" == "PASS" ]] || return 1
      if [[ "$(field fixture_source)" == "true" ]]; then
        [[ "$full_claim" == "false" ]] || return 1
        [[ "$(field controlled_live_proof_marker_present)" == "false" ]] || return 1
      else
        [[ "$full_claim" == "true" ]] || return 1
        [[ "$(field controlled_live_proof_marker_present)" == "true" ]] || return 1
        [[ "$(field controlled_proposal_created)" == "true" ]] || return 1
        [[ "$(field blocked_before_mutation)" == "false" ]] || return 1
        for key in provider_readiness_status messages_access_status selected_test_chat_status calendar_access_status reminders_access_status proposal_creation_status readback_status qa_action_observation_status reconcile_status idempotency_status privacy_inspection_status; do
          [[ "$(field "$key")" == "PASS" ]] || { printf 'FAIL validator: PASS missing %s\n' "$key" >&2; return 1; }
        done
      fi
      ;;
    BLOCKED)
      [[ "$full_claim" == "false" ]] || return 1
      [[ "$(field blocked_before_mutation)" == "true" || "$cleanup" == "PASS" ]] || return 1
      [[ -n "$(field blocker_reasons)" ]] || return 1
      ;;
    *) printf 'FAIL validator: invalid overall status\n' >&2; return 1 ;;
  esac
}

finalize_blocked() {
  ensure_surface_receipts
  write_cleanup "$3" "$4"
  write_privacy "PASS"
  write_summary "BLOCKED" "$1" "$2" "$3" "PASS"
  validate_receipt || exit 1
  cat "$summary_file"
  exit 0
}

finalize_fail() {
  ensure_surface_receipts
  write_cleanup "${cleanup_status:-BLOCKED_BEFORE_MUTATION}" "${cleanup_proof:-no_live_mutation_attempted}"
  write_privacy "${privacy_status:-PASS}"
  write_summary "FAIL" "$1" "${blocked_before_mutation:-true}" "${cleanup_status:-BLOCKED_BEFORE_MUTATION}" "${privacy_status:-PASS}"
  validate_receipt
  exit 1
}

fixture_receipt() {
  provider_status=PASS messages_status=PASS selected_chat_status=PASS
  calendar_status=PASS reminders_status=PASS proposal_status=PASS readback_status=PASS
  qa_action_status=PASS reconcile_status=PASS idempotency_status=PASS controlled_proposal_created=false
  controlled_live_proof_marker_present=false
  case "$fixture_case" in
    PASS) write_cleanup PASS controlled_fixture_cleanup; write_privacy PASS; write_summary PASS none false PASS PASS ;;
    BLOCKED) proposal_status=BLOCKED; controlled_proposal_created=false; write_cleanup BLOCKED_BEFORE_MUTATION no_live_mutation_attempted; write_privacy PASS; write_summary BLOCKED fixture_blocked_before_mutation true BLOCKED_BEFORE_MUTATION PASS ;;
    FAIL) provider_status=FAIL; write_cleanup BLOCKED_BEFORE_MUTATION no_live_mutation_attempted; write_privacy PASS; write_summary FAIL fixture_fail true BLOCKED_BEFORE_MUTATION PASS ;;
    MISSING_CLEANUP) write_privacy PASS; write_summary PASS none false PASS PASS ;;
    PROVIDER_JSON) write_cleanup PASS controlled_fixture_cleanup; write_privacy PASS; printf '%s\n' 'fixture_forbidden_payload_marker=present' > "$repo_root/$out_dir/provider-raw.txt"; write_summary PASS none false PASS PASS ;;
    MISSING_PRIVACY_SCAN) write_cleanup PASS controlled_fixture_cleanup; write_summary PASS none false PASS PASS ;;
  esac
  validate_receipt || exit 1
  cat "$summary_file"
}
