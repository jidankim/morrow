import path from "node:path";

import { sourceReceipt } from "./io.mjs";
import { boulderSnapshot, planState, receiptSummary } from "./parsers.mjs";
import { sourcePath } from "./sources.mjs";

export function sourceReceipts({ repoRoot, historicalRootInfo, historicalLabel }) {
  return [
    sourceReceipt("current_worktree", "README.md", path.join(repoRoot, "README.md")),
    sourceReceipt("current_worktree", ".omo/boulder.json", path.join(repoRoot, ".omo/boulder.json")),
    sourceReceipt(historicalLabel, ".omo/boulder.json", sourcePath(historicalRootInfo, ".omo/boulder.json")),
    sourceReceipt(
      "current_omo",
      ".omo/plans/messages-calendar-approval-trajectory-eval.md",
      path.join(repoRoot, ".omo/plans/messages-calendar-approval-trajectory-eval.md"),
    ),
    sourceReceipt(
      historicalLabel,
      ".omo/drafts/messages-calendar-approval-trajectory-eval.md",
      sourcePath(historicalRootInfo, ".omo/drafts/messages-calendar-approval-trajectory-eval.md"),
    ),
    sourceReceipt(
      historicalLabel,
      ".omo/plans/messages-calendar-production-wiring.md",
      sourcePath(historicalRootInfo, ".omo/plans/messages-calendar-production-wiring.md"),
    ),
    sourceReceipt(
      "current_worktree",
      "src-tauri/src/native_bridge/codex_provider/mod.rs",
      path.join(repoRoot, "src-tauri/src/native_bridge/codex_provider/mod.rs"),
    ),
    sourceReceipt(
      "current_worktree",
      "scripts/messages-calendar-real-qa.sh",
      path.join(repoRoot, "scripts/messages-calendar-real-qa.sh"),
    ),
  ];
}

export function evidenceReceipts({ historicalLabel, historicalRootInfo, parsers }) {
  return {
    phase3Local: receiptSummary(
      historicalLabel,
      ".omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/summary.txt",
      sourcePath(historicalRootInfo, ".omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/summary.txt"),
      parsers.parsePhaseSmoke,
    ),
    phase3Real: receiptSummary(
      historicalLabel,
      ".omo/evidence/phase-3-lifecycle-replay-coverage/real-surface/summary.txt",
      sourcePath(historicalRootInfo, ".omo/evidence/phase-3-lifecycle-replay-coverage/real-surface/summary.txt"),
      parsers.parseRealSurface,
    ),
    phase4Local: receiptSummary(
      historicalLabel,
      ".omo/evidence/phase-4-human-approval-correction/final-smoke/summary.txt",
      sourcePath(historicalRootInfo, ".omo/evidence/phase-4-human-approval-correction/final-smoke/summary.txt"),
      parsers.parsePhaseSmoke,
    ),
    phase4Real: receiptSummary(
      historicalLabel,
      ".omo/evidence/phase-4-human-approval-correction/real-surface/summary.txt",
      sourcePath(historicalRootInfo, ".omo/evidence/phase-4-human-approval-correction/real-surface/summary.txt"),
      parsers.parseRealSurface,
    ),
    task7: receiptSummary(
      historicalLabel,
      ".omo/evidence/messages-calendar-production-wiring/task-7/real-qa-run.log",
      sourcePath(historicalRootInfo, ".omo/evidence/messages-calendar-production-wiring/task-7/real-qa-run.log"),
      parsers.parseMessagesCalendarTask7,
    ),
  };
}

export function predecessorPlanState(repoRoot, historicalLabel, historicalRootInfo) {
  return {
    approval_trajectory_eval: planState(
      "current_omo",
      ".omo/plans/messages-calendar-approval-trajectory-eval.md",
      path.join(repoRoot, ".omo/plans/messages-calendar-approval-trajectory-eval.md"),
      "Add the Phase 5 preflight",
    ),
    messages_calendar_production_wiring: planState(
      historicalLabel,
      ".omo/plans/messages-calendar-production-wiring.md",
      sourcePath(historicalRootInfo, ".omo/plans/messages-calendar-production-wiring.md"),
      "Real Messages-to-Calendar QA runner",
    ),
    lifecycle_replay_coverage: planState(
      historicalLabel,
      ".omo/plans/lifecycle-replay-coverage.md",
      sourcePath(historicalRootInfo, ".omo/plans/lifecycle-replay-coverage.md"),
    ),
    human_approval_correction_loop: planState(
      historicalLabel,
      ".omo/plans/human-approval-correction-loop.md",
      sourcePath(historicalRootInfo, ".omo/plans/human-approval-correction-loop.md"),
    ),
  };
}

export function boulderSnapshots(repoRoot, historicalLabel, historicalRootInfo) {
  const currentBoulderPath = path.join(repoRoot, ".omo/boulder.json");
  const historicalBoulderPath = sourcePath(historicalRootInfo, ".omo/boulder.json");
  return {
    current: boulderSnapshot("current_omo", ".omo/boulder.json", currentBoulderPath),
    historical: boulderSnapshot(historicalLabel, ".omo/boulder.json", historicalBoulderPath),
  };
}

export function buildReport(context) {
  const receipts = context.receipts;
  return {
    schema: "phase5_messages_calendar_approval_trajectory_preflight_v1",
    run_id: context.runId,
    generated_at_utc: context.generatedAt,
    invocation:
      "node scripts/messages-calendar-approval-trajectory-preflight.mjs --out-dir .omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/preflight",
    current_run_marker: true,
    overall_status: context.overallStatus,
    phase3_local_smoke: receipts.phase3Local.status,
    phase3_real_surface: receipts.phase3Real.status,
    phase4_local_smoke: receipts.phase4Local.status,
    phase4_real_surface: receipts.phase4Real.status,
    messages_calendar_live: context.liveGate.status,
    live_claim_allowed: context.liveGate.live_claim_allowed,
    git_status_porcelain_v1_untracked_all: context.gitStatus,
    dirty_worktree_allowlist: context.dirtyAllowlist,
    boulder_snapshot: context.boulderSnapshot,
    predecessor_plan_checkbox_state: context.predecessorPlanState,
    evidence_receipts: {
      phase3_local_smoke: receipts.phase3Local,
      phase3_real_surface: receipts.phase3Real,
      phase4_local_smoke: receipts.phase4Local,
      phase4_real_surface: receipts.phase4Real,
      messages_calendar_task_7: receipts.task7,
    },
    provider_auth_docs: context.docs,
    provider_auth_resolution: context.resolution,
    live_gate: context.liveGate,
    historical_omo_discovery: context.historicalDiscovery,
    source_freshness: {
      method: "current_run_marker_plus_source_stat_and_sha256",
      source_receipts: context.sourceReceipts,
    },
    fixture_provider_conflict: context.fixtureResult,
    privacy: {
      report_is_sanitized: true,
      raw_messages_included: false,
      provider_payloads_included: false,
      secrets_included: false,
      native_ids_included: false,
      app_data_paths_included: false,
    },
  };
}
