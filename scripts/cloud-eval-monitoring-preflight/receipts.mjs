import path from "node:path";

import { exists, readText } from "./io.mjs";
import { rootLabel, sourcePath } from "./sources.mjs";

const priorPhaseReceiptPaths = [
  ["phase1_production_trace_sink", ".omo/evidence/phase-1-production-trace-sink/final-smoke/summary.txt"],
  ["phase2_trace_candidate_correlation", ".omo/evidence/phase-2-trace-candidate-correlation/final-smoke/summary.txt"],
  ["phase3_lifecycle_replay_final_smoke", ".omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/summary.txt"],
  ["phase3_lifecycle_real_surface", ".omo/evidence/phase-3-lifecycle-replay-coverage/real-surface/summary.txt"],
  ["phase4_human_approval_final_smoke", ".omo/evidence/phase-4-human-approval-correction/final-smoke/summary.txt"],
  ["phase4_human_approval_real_surface", ".omo/evidence/phase-4-human-approval-correction/real-surface/summary.txt"],
  ["phase5_trajectory_final_smoke", ".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke/summary.txt"],
  ["phase5_trajectory_live_receipt", ".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/live-receipt/summary.txt"],
];

const cloudPrerequisitePaths = [
  ["telemetry_contract", ".omo/evidence/cloud-telemetry-contract/final-smoke/summary.txt"],
  ["collector", ".omo/evidence/cloud-collector/final-smoke/summary.txt"],
  ["collector_alt", ".omo/evidence/cloud-telemetry-collector/final-smoke/summary.txt"],
  ["deletion_retention", ".omo/evidence/cloud-deletion-retention/final-smoke/summary.txt"],
  ["retention_audit", ".omo/evidence/cloud-retention-audit/final-smoke/summary.txt"],
];

export function priorPhaseReceipts(repoRoot, receiptRootInfo) {
  const label = rootLabel(receiptRootInfo, repoRoot);
  const receipts = {};
  for (const [name, relativePath] of priorPhaseReceiptPaths) {
    receipts[name] = textReceipt(label, relativePath, sourcePath(receiptRootInfo, relativePath));
  }
  return receipts;
}

export function cloudPrerequisiteReceipts(repoRoot, receiptRootInfo, mode) {
  const label = rootLabel(receiptRootInfo, repoRoot);
  const receipts = {};
  for (const [name, relativePath] of cloudPrerequisitePaths) {
    const receipt = textReceipt(label, relativePath, sourcePath(receiptRootInfo, relativePath));
    receipts[name] = receipt.present
      ? receipt
      : {
          ...receipt,
          status: mode === "fixture_smoke" ? "not_required_for_fixture_smoke" : "MISSING",
        };
  }
  return receipts;
}

export function phase5FinalSmoke(priorReceipts) {
  return priorReceipts.phase5_trajectory_final_smoke;
}

export function malformedReceipts(...receiptGroups) {
  return receiptGroups
    .flatMap((group) => Object.entries(group))
    .filter(([, receipt]) => receipt.present && receipt.status === "MALFORMED")
    .map(([name, receipt]) => ({ name, source: receipt.source, reason: receipt.malformed_reason }));
}

function textReceipt(root, relativePath, absolutePath) {
  const source = { root, path: relativePath };
  if (!exists(absolutePath)) {
    return { source, present: false, status: "MISSING" };
  }
  const text = readText(absolutePath);
  const status = parseStatus(text);
  const malformedReason = status === "MALFORMED" ? "present receipt lacks PASS/BLOCKED/FAIL result marker" : null;
  return {
    source,
    present: true,
    status,
    invocation: firstMatch(text, /^(?:invocation|command):\s*(.+)$/m),
    cleanup_receipt_present: /cleanup[_-]receipt[:=]\s*\S+/i.test(text),
    privacy_receipt_present: /privacy[_-](?:inspect|inspection)[:=]\s*\S+/i.test(text),
    canary_rejection_present: /canary[_-]rejection[:=]\s*\S+/i.test(text),
    no_live_backend_observed:
      /no live backend|live_backend=absent|backend observable says no live/i.test(text) ||
      /no live backend\/provider\/network\/native surfaces\/vendor backend/i.test(text),
    malformed_reason: malformedReason,
  };
}

function parseStatus(text) {
  if (/^result:\s*PASS$/m.test(text) || /^overall_status[=:]PASS$/m.test(text) || /^PASS\b/m.test(text)) {
    return "PASS";
  }
  if (/^result:\s*BLOCKED$/m.test(text) || /^overall_status[=:]BLOCKED$/m.test(text) || /^BLOCKED\b/m.test(text)) {
    return "BLOCKED";
  }
  if (/^result:\s*FAIL$/m.test(text) || /^overall_status[=:]FAIL$/m.test(text) || /^FAIL\b/m.test(text)) {
    return "FAIL";
  }
  return "MALFORMED";
}

function firstMatch(text, pattern) {
  const match = text.match(pattern);
  return match ? match[1] : null;
}
