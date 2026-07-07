export function buildMarkdown(report) {
  const dirty = report.dirty_worktree_classification;
  const beta = report.beta_prerequisites;
  const phase6 = report.phase6_fixture_monitoring;
  return `# Phase 7 Wider Product Rollout Preflight

phase: ${report.phase}
scope: ${report.scope}
status: ${report.status}
rollout_source_of_truth: ${report.rollout_source_of_truth}
remote_config: ${report.remote_config}
generated_at_utc: ${report.generated_at_utc}
run_id: ${report.run_id}

## Source Of Truth
- rollout: static local manifest only
- beta_plan_source_of_truth: ${report.beta_plan_source_of_truth}
- docs_mirror_policy: ${beta.docs_mirror_policy}
- public/assets/morrow-hero.png: unrelated unless a later Phase 7 receipt intentionally uses it

## Phase 6 Boundary
- status: ${phase6.status}
- deployed_rollout_evidence: ${phase6.deployed_rollout_evidence}
- release_gate_overall_status: ${phase6.release_gate_overall_status}
- deployment_action: ${phase6.release_gate_deployment_action}
- backend_observable_no_live_surface: ${phase6.backend_observable_no_live_surface}

## Beta Prerequisites
- status: ${beta.status}
- default_availability_allowed: ${beta.default_availability_allowed}
- missing_count: ${beta.missing_prerequisites.length}

## Privacy Defaults
- telemetry_hard_off: ${report.privacy_config_defaults.telemetry_hard_off}
- crash_log_excerpts_hard_off: ${report.privacy_config_defaults.crash_log_excerpts_hard_off}
- local_diagnostics_default_enabled: ${report.privacy_config_defaults.local_diagnostics_default_enabled}
- local_diagnostics_retention_days_default: ${report.privacy_config_defaults.local_diagnostics_retention_days_default}

## Dirty Worktree Classification
- status: ${dirty.status}
- entry_count: ${dirty.entry_count}
- product_runtime_entry_count: ${dirty.product_runtime_entries.length}
- planning_evidence_entry_count: ${dirty.planning_evidence_entries.length}

## Product Vs Evidence Retention Boundary
- product_runtime_artifacts: ${report.product_vs_evidence_retention_boundary.product_runtime_artifacts}
- planning_evidence_artifacts: ${report.product_vs_evidence_retention_boundary.planning_evidence_artifacts}
- delete_all_boundary: ${report.product_vs_evidence_retention_boundary.delete_all_boundary}

## Forbidden Raw Content
- status: ${report.forbidden_raw_content.status}
- hits: ${report.forbidden_raw_content.hits.length}

## Adversarial Notes
- malformed_input: contradictory fixture for Phase 6 deployed rollout exits nonzero.
- stale_state: generated_at_utc, run_id, source mtimes, and hashes are rewritten on each run.
- dirty_worktree: current porcelain status is classified and does not hide other workers' files.
- misleading_success_output: validators inspect JSON and Markdown fields, not only exit code.
- hung_or_long_commands: the only subprocess is bounded local git status.
- flaky_tests: no timing-sensitive test logic was added.
- prompt_injection: repository docs and receipts are consumed as inert text and never executed.
- cancel_resume: no resumable flow exists in this narrow receipt writer.
- repeated_interruptions: no persistent process is started.
`;
}
