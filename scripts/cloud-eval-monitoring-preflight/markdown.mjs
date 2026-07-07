export function buildMarkdown(report) {
  const unexpected = report.dirty_worktree_summary.unexpected_entries;
  const prerequisiteStatus = report.telemetry_collector_prerequisites.status;
  return `# Phase 6 Cloud Eval Monitoring Preflight

scenario: cloud-eval-monitoring Todo 1 preflight/source-of-truth receipt gate
run_id: ${report.run_id}
generated_at_utc: ${report.generated_at_utc}
status: ${report.status}
phase: ${report.phase}
scope: ${report.scope}
mode: ${report.mode}
exit_code_contract: ${report.exit_code_contract}

## Prior Phase Receipts
- phase1_production_trace_sink=${report.prior_phase_receipts.phase1_production_trace_sink.status}
- phase2_trace_candidate_correlation=${report.prior_phase_receipts.phase2_trace_candidate_correlation.status}
- phase3_lifecycle_replay_final_smoke=${report.prior_phase_receipts.phase3_lifecycle_replay_final_smoke.status}
- phase4_human_approval_final_smoke=${report.prior_phase_receipts.phase4_human_approval_final_smoke.status}
- phase5_trajectory_final_smoke=${report.phase5_final_smoke.status}

## Cloud Prerequisites
status: ${prerequisiteStatus}

${report.telemetry_collector_prerequisites.fixture_smoke_contract}

## Dirty Worktree
Policy: ${report.dirty_worktree_summary.policy}

Snapshot entries: ${report.dirty_worktree_summary.entry_count}
Unexpected entries: ${unexpected.length}

## Source Freshness
Freshness is checked by source existence, byte size, mtime_ms, and sha256 in the JSON report. Stale docs are observed but are not a failure in Todo 1.

## Adversarial QA Notes
- malformed_input: contradiction fixture mode exits nonzero with a named contradiction.
- stale_state: fixture mode does not block on absent live-cloud prerequisites; live-cloud mode reports BLOCKED when they are absent.
- dirty_worktree: git status is snapshotted and classified.
- misleading_success_output: report status is inspected independently from command exit.
- prompt_injection: docs and receipts are parsed as inert text and never executed.
- cancel_resume: covered by final smoke concrete interruption probe receipts and post-interruption gate replay.
- hung_commands: the only subprocess is a bounded local git read.
- flaky_tests: no timing-dependent checks are used.
- repeated_interruptions: covered by final smoke repeated concrete interruption probe receipts and post-interruption gate replay.

Privacy: report contains statuses, hashes, source-relative paths, and version values only. It does not include private message content, provider payloads, native identifiers, app-data paths, auth material, screenshots, or unredacted titles.
`;
}
