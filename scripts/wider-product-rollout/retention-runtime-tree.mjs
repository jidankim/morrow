import fs from "node:fs";
import path from "node:path";

import { daySeconds, fixedNowSeconds, runtimeDiagnosticsDirs } from "./retention-common.mjs";

export function createRuntimeTree(outDir) {
  const runtimeRoot = path.join(outDir, "runtime-tree");
  const appData = path.join(runtimeRoot, "app-data");
  const tracesDir = path.join(appData, "diagnostics", "traces");
  const evalsDir = path.join(appData, "diagnostics", "evals");
  const exportsDir = path.join(appData, "diagnostics", "exports");
  const manualExportDir = path.join(appData, "diagnostics", "manual-exports");
  const diagnosticsSibling = path.join(appData, "diagnostics-user-export");
  const unrelatedSibling = path.join(runtimeRoot, "sibling-user-files");
  const globalCodexDir = path.join(runtimeRoot, "home", ".codex");
  const planningEvidenceDir = path.join(runtimeRoot, ".omo", "evidence", "phase-7-wider-product-rollout", "task-5");
  const providerCredentialDir = path.join(appData, "provider-credentials");

  for (const dir of [
    tracesDir,
    evalsDir,
    exportsDir,
    manualExportDir,
    diagnosticsSibling,
    unrelatedSibling,
    globalCodexDir,
    planningEvidenceDir,
    providerCredentialDir,
  ]) {
    fs.mkdirSync(dir, { recursive: true });
  }

  const staleName = `trace-${fixedNowSeconds - 31 * daySeconds}-0.jsonl`;
  const boundaryName = `trace-${fixedNowSeconds - 30 * daySeconds}-0.jsonl`;
  const freshName = `trace-${fixedNowSeconds - daySeconds}-0.jsonl`;
  fs.writeFileSync(path.join(tracesDir, staleName), "{\"trace_id\":\"stale\"}\n");
  fs.writeFileSync(path.join(tracesDir, boundaryName), "{\"trace_id\":\"boundary\"}\n");
  fs.writeFileSync(path.join(tracesDir, freshName), "{\"trace_id\":\"fresh\"}\n");
  fs.writeFileSync(path.join(tracesDir, "notes.txt"), "ignored non-trace file\n");
  fs.writeFileSync(path.join(evalsDir, "eval.jsonl"), "{\"metric\":\"metadata_only\"}\n");
  fs.writeFileSync(path.join(exportsDir, "export.json"), "{\"schema\":\"sanitized\"}\n");
  fs.writeFileSync(path.join(manualExportDir, "manual-export.json"), "{\"owner\":\"user\"}\n");
  fs.writeFileSync(path.join(diagnosticsSibling, "user-export.json"), "{\"owner\":\"user\"}\n");
  fs.writeFileSync(path.join(unrelatedSibling, "notes.txt"), "user file\n");
  fs.writeFileSync(path.join(globalCodexDir, "auth.json"), "{\"note\":\"global codex login sentinel\"}\n");
  fs.writeFileSync(
    path.join(planningEvidenceDir, "planning-evidence.md"),
    "provider_api_key: <redacted>\nsummary: sanitized planning evidence\n"
  );
  fs.writeFileSync(
    path.join(providerCredentialDir, "morrow-openai-provider-api-key.marker"),
    "legacy Morrow-owned provider marker\n"
  );
  fs.writeFileSync(path.join(appData, "morrow.sqlite"), "sqlite placeholder\n");

  return {
    runtimeRoot,
    appData,
    tracesDir,
    providerCredentialDir,
    sentinels: {
      staleTrace: `app-data/diagnostics/traces/${staleName}`,
      boundaryTrace: `app-data/diagnostics/traces/${boundaryName}`,
      freshTrace: `app-data/diagnostics/traces/${freshName}`,
      manualExport: "app-data/diagnostics/manual-exports/manual-export.json",
      diagnosticsSibling: "app-data/diagnostics-user-export/user-export.json",
      unrelatedSibling: "sibling-user-files/notes.txt",
      globalCodexLogin: "home/.codex/auth.json",
      planningEvidence: ".omo/evidence/phase-7-wider-product-rollout/task-5/planning-evidence.md",
      providerMarker: "app-data/provider-credentials/morrow-openai-provider-api-key.marker",
      database: "app-data/morrow.sqlite",
    },
  };
}

export function simulateDeleteAll(tree) {
  const deletedPaths = [];
  const removePath = (relativePath) => {
    const absolutePath = path.join(tree.runtimeRoot, relativePath);
    if (!fs.existsSync(absolutePath)) {
      return false;
    }
    fs.rmSync(absolutePath, { recursive: true, force: true });
    deletedPaths.push(relativePath);
    return true;
  };

  const databaseDeleted = removePath("app-data/morrow.sqlite");
  let diagnosticsArtifactsDeleted = false;
  for (const diagnosticsDir of runtimeDiagnosticsDirs) {
    diagnosticsArtifactsDeleted = removePath(`app-data/${diagnosticsDir}`) || diagnosticsArtifactsDeleted;
  }
  const providerMarkerDeleted = removePath("app-data/provider-credentials/morrow-openai-provider-api-key.marker");

  return {
    receipt: {
      storageSurface: "morrowStore",
      databaseDeleted,
      approvedExternalItemsDeleted: false,
      diagnosticsArtifactsDeleted,
      providerOAuthDeleteRequested: true,
      providerOAuthDeleted: providerMarkerDeleted,
      providerOAuthDeleteFailed: false,
      providerCredentialDeletes: [
        {
          tokenKind: "morrow-openai-provider-api-key",
          deleteRequested: true,
          deleted: providerMarkerDeleted,
          failed: false,
        },
      ],
      cleanupPlan: {
        proposedItems: "completed",
        emptyProposalContainers: "skippedByUser",
        proposedCalendarItemsDeleted: 0,
        proposedReminderItemsDeleted: 0,
      },
    },
    deletedPaths: deletedPaths.sort(),
  };
}
