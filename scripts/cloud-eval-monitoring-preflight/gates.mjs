import path from "node:path";

import { readJson } from "./io.mjs";

export function dirtyWorktreeSummary(gitStatus) {
  const allowedPrefixes = [
    "scripts/cloud-eval-monitoring-docs-qa/",
    "scripts/cloud-eval-monitoring-fixtures/",
    "scripts/cloud-eval-monitoring-preflight/",
    "scripts/cloud-eval-monitoring-report/",
    "scripts/cloud-eval-monitoring-smoke/",
  ];
  const allowedExact = new Set([
    "README.md",
    "docs/architecture/sans-io-boundaries.md",
    "docs/axi-phase-0-feedback-eval-baseline.md",
    "docs/diagnostics-trace-eval.md",
    "scripts/cloud-eval-monitoring-dashboard.mjs",
    "scripts/cloud-eval-monitoring-docs-qa.mjs",
    "scripts/cloud-eval-monitoring-gates.mjs",
    "scripts/cloud-eval-monitoring-metrics.mjs",
    "scripts/cloud-eval-monitoring-preflight.mjs",
    "scripts/cloud-eval-monitoring-report-generate.mjs",
    "scripts/cloud-eval-monitoring-report-lib.mjs",
    "scripts/cloud-eval-monitoring-report.mjs",
    "scripts/run-cloud-eval-monitoring-smoke.sh",
  ]);
  const entries = gitStatus
    .split("\n")
    .filter((line) => line.trim().length > 0)
    .map((line) => {
      const status = line.slice(0, 2);
      const filePath = line.slice(3);
      const allowed =
        allowedExact.has(filePath) ||
        allowedPrefixes.some((prefix) => filePath.startsWith(prefix));
      return { status, path: filePath, allowed };
    });
  return {
    policy:
      "Allowed dirty entries are the known Phase 6 cloud eval monitoring docs, named root scripts, and exact script subdirectories for docs QA, fixtures, preflight, report modules, and smoke helpers.",
    entries,
    entry_count: entries.length,
    unexpected_entries: entries.filter((entry) => !entry.allowed),
    all_entries_allowed: entries.every((entry) => entry.allowed),
  };
}

export function validateContradictionFixture(fixturePath) {
  if (!fixturePath) {
    return null;
  }
  const fixture = readJson(path.resolve(fixturePath));
  const docsSayFuture =
    fixture.docs_cloud_telemetry_future_work === true ||
    /future work/i.test(String(fixture.current_docs_claim ?? ""));
  const claimsLiveCollectorPass =
    fixture.claimed_live_cloud_collector_status === "PASS" ||
    fixture.claimed_collector_receipt_status === "PASS";
  const note = String(fixture.reconciliation_note ?? "").trim();
  const noteIsExplicit =
    note.length > 0 && /cloud/i.test(note) && /(future work|historical|reconciled)/i.test(note);
  const contradiction = docsSayFuture && claimsLiveCollectorPass && !noteIsExplicit;
  return {
    fixture_path: fixturePath,
    status: contradiction ? "PROVIDER_SOURCE_OF_TRUTH_CONTRADICTION" : "PASS",
    contradiction_name: contradiction ? "cloud_collector_receipt_conflicts_with_current_docs" : null,
    docs_cloud_telemetry_future_work: docsSayFuture,
    claimed_live_cloud_collector_status: claimsLiveCollectorPass ? "PASS" : "not_pass",
    explicit_reconciliation_note_present: noteIsExplicit,
  };
}

export function prerequisiteSummary({ mode, cloudPrerequisites }) {
  const receipts = Object.entries(cloudPrerequisites).map(([name, receipt]) => ({
    name,
    status: receipt.status,
    present: receipt.present,
    source: receipt.source,
  }));
  const missing = receipts.filter((receipt) => !receipt.present);
  const blocked = mode === "live_cloud_input" && missing.length > 0;
  return {
    mode,
    status: blocked ? "BLOCKED" : "PASS",
    missing_count: missing.length,
    receipts,
    live_cloud_input_allowed: !blocked,
    fixture_smoke_contract:
      mode === "fixture_smoke"
        ? "Missing telemetry, collector, deletion, and retention receipts are recorded as not_required_for_fixture_smoke."
        : "Live cloud input requires telemetry, collector, deletion, and retention prerequisite receipts.",
  };
}

export function overallStatus({ contradiction, malformedReceiptCount, dirtySummary, prereqSummary }) {
  if (contradiction?.status === "PROVIDER_SOURCE_OF_TRUTH_CONTRADICTION") {
    return "FAIL";
  }
  if (malformedReceiptCount > 0) {
    return "FAIL";
  }
  if (!dirtySummary.all_entries_allowed) {
    return "FAIL";
  }
  if (prereqSummary.status === "BLOCKED") {
    return "BLOCKED";
  }
  return "PASS";
}
