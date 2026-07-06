export function buildMarkdown(report) {
  const unexpected = report.dirty_worktree_allowlist.unexpected_entries;
  return `# Phase 5 Preflight Report

scenario: messages-calendar-approval-trajectory-eval Todo 1 preflight/source-of-truth gate
run_id: ${report.run_id}
generated_at_utc: ${report.generated_at_utc}
overall_status: ${report.overall_status}

## Gate Summary
- phase3_local_smoke=${report.phase3_local_smoke}
- phase3_real_surface=${report.phase3_real_surface}
- phase4_local_smoke=${report.phase4_local_smoke}
- phase4_real_surface=${report.phase4_real_surface}
- messages_calendar_live=${report.messages_calendar_live}
- live_claim_allowed=${report.live_claim_allowed}
- provider_auth_resolution=${report.provider_auth_resolution.status}

## Provider Auth
Current source of truth: ${report.provider_auth_resolution.current_source_of_truth}

Resolution: ${report.provider_auth_resolution.explicit_reconciliation_note}

## Dirty Worktree
Policy: ${report.dirty_worktree_allowlist.policy}

Snapshot entries: ${report.dirty_worktree_allowlist.entries.length}
Unexpected entries: ${unexpected.length}

## Source Freshness
Freshness is checked by current-run marker plus source existence, byte size, mtime_ms, and sha256 in the JSON report. No wall-clock comparison is required for pass/fail.

## ULTRAQA Notes
- malformed_input: covered by provider-conflict fixture mode; contradictory fixtures exit nonzero.
- stale_state: JSON includes run_id, generated_at_utc, source paths, sizes, mtimes, and hashes.
- dirty_worktree: git status is snapshotted and classified through dirty_worktree_allowlist.
- misleading_success_output: overall_status is BLOCKED when dirty-worktree blockers are present.
- untrusted_external_text_prompt_injection: docs/evidence are parsed as inert text and never executed.
- cancel_resume: not applicable; the script is a single non-resumable read/report command.
- hung_long_commands: not applicable; the only subprocesses are bounded local git reads.
- flaky_tests: freshness uses hashes/current-run markers rather than brittle timestamp thresholds.
- repeated_interruptions: not applicable; no mid-operation interrupt protocol was added.

Privacy: report contains statuses, hashes, and source-relative paths only. It does not include raw Messages bodies, provider prompts/responses/JSON, native IDs, app-data paths, unredacted titles, handles, emails, phone numbers, secrets, tokens, cookies, or screenshots.
`;
}
