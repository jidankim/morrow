import path from "node:path";

import {
  addCheck,
  applyRetention,
  defaultRetentionDays,
  fixedNowSeconds,
  listFiles,
  retentionDaysIsAccepted,
  scanPlanningEvidence,
  writeJson,
} from "./retention-common.mjs";

export function addRetentionValueChecks(checks) {
  for (const value of [1, 30, 365]) {
    addCheck(
      checks,
      retentionDaysIsAccepted(value) ? "PASS" : "FAIL",
      `config_accepts_${value}_days`,
      "1, 30, and 365 day retention values are accepted",
      `value ${value} accepted=${retentionDaysIsAccepted(value)}`,
      "script runtime validator"
    );
  }
  for (const value of [0, 366, "30"]) {
    const valueLabel = typeof value === "string" ? `string_${value}` : String(value);
    addCheck(
      checks,
      retentionDaysIsAccepted(value) ? "FAIL" : "PASS",
      `config_rejects_${valueLabel}_days`,
      "0, 366, and string retention values are rejected",
      `value ${JSON.stringify(value)} accepted=${retentionDaysIsAccepted(value)}`,
      "script runtime validator"
    );
  }
}

export function runRetentionScenario(checks, tree, outDir, outDirArg) {
  const beforeRetention = listFiles(tree.runtimeRoot);
  writeJson(path.join(outDir, "retention-before.json"), {
    fixed_now_seconds: fixedNowSeconds,
    retention_days: defaultRetentionDays,
    files: beforeRetention,
  });
  const removedByRetention = applyRetention(tree.tracesDir, defaultRetentionDays, fixedNowSeconds);
  const afterRetention = listFiles(tree.runtimeRoot);
  writeJson(path.join(outDir, "retention-after.json"), {
    fixed_now_seconds: fixedNowSeconds,
    retention_days: defaultRetentionDays,
    removed_by_retention: removedByRetention,
    files: afterRetention,
  });
  addCheck(
    checks,
    removedByRetention.length === 1 &&
      afterRetention.includes(tree.sentinels.boundaryTrace) &&
      afterRetention.includes(tree.sentinels.freshTrace) &&
      afterRetention.includes("app-data/diagnostics/traces/notes.txt")
      ? "PASS"
      : "FAIL",
    "runtime_retention_cleanup",
    "stale runtime diagnostics cleanup removes older-than-retention trace files only",
    `removed=${removedByRetention.join(",") || "none"}`,
    path.join(outDirArg, "retention-after.json"),
    { before_count: beforeRetention.length, after_count: afterRetention.length }
  );
}

export function runDeleteAllScenario(checks, tree, deleteResult, outDir, outDirArg) {
  const afterDelete = listFiles(tree.runtimeRoot);
  writeJson(path.join(outDir, "delete-receipt.json"), {
    deleted_paths: deleteResult.deletedPaths,
    receipt: deleteResult.receipt,
    remaining_files: afterDelete,
  });

  const expectedDeleted = [
    "app-data/diagnostics/evals",
    "app-data/diagnostics/exports",
    "app-data/diagnostics/traces",
    "app-data/morrow.sqlite",
    "app-data/provider-credentials/morrow-openai-provider-api-key.marker",
  ];
  const deletedExactlyExpected = JSON.stringify(deleteResult.deletedPaths) === JSON.stringify(expectedDeleted);
  const protectedFilesRemain = [
    tree.sentinels.manualExport,
    tree.sentinels.diagnosticsSibling,
    tree.sentinels.unrelatedSibling,
    tree.sentinels.globalCodexLogin,
    tree.sentinels.planningEvidence,
  ].every((relativePath) => afterDelete.includes(relativePath));
  addCheck(
    checks,
    deletedExactlyExpected && protectedFilesRemain ? "PASS" : "FAIL",
    "delete_all_bounded_paths",
    "Delete All deletes Morrow diagnostics and legacy provider marker, not sibling/user/global/planning files",
    `deleted=${deleteResult.deletedPaths.join(",")}`,
    path.join(outDirArg, "delete-receipt.json"),
    { protected_files_remain: protectedFilesRemain }
  );
  addCheck(
    checks,
    deleteResult.receipt.diagnosticsArtifactsDeleted === true &&
      deleteResult.receipt.providerCredentialDeletes.length === 1 &&
      deleteResult.receipt.providerCredentialDeletes[0].tokenKind === "morrow-openai-provider-api-key" &&
      deleteResult.receipt.providerOAuthDeleted === true
      ? "PASS"
      : "FAIL",
    "delete_all_receipt_fields",
    "Delete All receipt includes diagnosticsArtifactsDeleted and provider credential cleanup details",
    JSON.stringify(deleteResult.receipt),
    path.join(outDirArg, "delete-receipt.json")
  );
  return afterDelete;
}

export function runPlanningEvidenceScenario(checks, tree, afterDelete, outDir, outDirArg) {
  const planningEvidencePath = path.join(tree.runtimeRoot, tree.sentinels.planningEvidence);
  const planningEvidenceViolations = scanPlanningEvidence(planningEvidencePath);
  writeJson(path.join(outDir, "planning-evidence-scan.json"), {
    scanned_path: tree.sentinels.planningEvidence,
    result: planningEvidenceViolations.length === 0 ? "PASS" : "FAIL",
    violations: planningEvidenceViolations,
  });
  addCheck(
    checks,
    planningEvidenceViolations.length === 0 && afterDelete.includes(tree.sentinels.planningEvidence) ? "PASS" : "FAIL",
    "planning_evidence_privacy_scanned_not_deleted",
    ".omo/evidence is local planning evidence: privacy-scanned and sanitized, not product-deleted",
    `planning-evidence violations=${planningEvidenceViolations.join(",") || "none"}`,
    path.join(outDirArg, "planning-evidence-scan.json")
  );
}
