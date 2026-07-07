export function buildReport(context) {
  return {
    schema: "phase6_cloud_eval_monitoring_preflight_v1",
    run_id: context.runId,
    generated_at_utc: context.generatedAt,
    status: context.status,
    phase: 6,
    scope: "cloud_eval_monitoring",
    mode: context.mode,
    invocation: context.invocation,
    exit_code_contract:
      "PASS and BLOCKED write reports and exit 0; FAIL writes a report when possible and exits nonzero.",
    current_run_marker: true,
    docs_source_of_truth: context.docs,
    package_app_versions: context.versionSources,
    git_status_porcelain_v1_untracked_all: context.gitStatus,
    dirty_worktree_summary: context.dirtySummary,
    prior_phase_receipts: context.priorReceipts,
    phase5_final_smoke: context.phase5FinalSmoke,
    telemetry_collector_prerequisites: context.prereqSummary,
    malformed_receipts: context.malformedReceipts,
    contradiction_fixture: context.contradictionFixture,
    receipt_root_discovery: context.receiptRootDiscovery,
    source_freshness: {
      method: "source_stat_and_sha256",
      source_receipts: context.sourceReceipts,
    },
    privacy: context.privacy,
  };
}
