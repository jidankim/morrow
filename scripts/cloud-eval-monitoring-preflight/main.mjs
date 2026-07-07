import process from "node:process";
import path from "node:path";

import { parseArgs, reportJsonName, reportMdName } from "./cli.mjs";
import {
  dirtyWorktreeSummary,
  overallStatus,
  prerequisiteSummary,
  validateContradictionFixture,
} from "./gates.mjs";
import { readText, runGit, writeReportFiles } from "./io.mjs";
import { buildMarkdown } from "./markdown.mjs";
import { assertSanitized } from "./privacy.mjs";
import {
  cloudPrerequisiteReceipts,
  malformedReceipts,
  phase5FinalSmoke,
  priorPhaseReceipts,
} from "./receipts.mjs";
import { buildReport } from "./report.mjs";
import {
  currentDocs,
  discoverReceiptRoot,
  sourceReceipts,
} from "./sources.mjs";
import { versionSources } from "./versions.mjs";

export function main() {
  const repoRoot = process.cwd();
  const args = parseArgs(process.argv.slice(2), repoRoot);
  const mode = args.requireCloudPrerequisites ? "live_cloud_input" : "fixture_smoke";
  const generatedAt = new Date().toISOString();
  const runId = `phase6-preflight-${generatedAt.replace(/[^0-9TZ]/g, "")}-${process.pid}`;
  const gitStatus = args.fixtureGitStatus
    ? readText(path.resolve(repoRoot, args.fixtureGitStatus))
    : runGit(["status", "--porcelain=v1", "--untracked-files=all"]);
  const receiptRootInfo = discoverReceiptRoot(repoRoot);
  const priorReceipts = priorPhaseReceipts(repoRoot, receiptRootInfo);
  const cloudPrereqs = cloudPrerequisiteReceipts(repoRoot, receiptRootInfo, mode);
  const badReceipts = malformedReceipts(priorReceipts, cloudPrereqs);
  const dirtySummary = dirtyWorktreeSummary(gitStatus);
  const prereqSummary = prerequisiteSummary({ mode, cloudPrerequisites: cloudPrereqs });
  const contradictionFixture = validateContradictionFixture(args.fixtureContradiction);
  const status = overallStatus({
    contradiction: contradictionFixture,
    malformedReceiptCount: badReceipts.length,
    dirtySummary,
    prereqSummary,
  });
  let report = buildReport({
    runId,
    generatedAt,
    status,
    mode,
    invocation: `node scripts/cloud-eval-monitoring-preflight.mjs --out-dir ${args.outDir}${
      args.requireCloudPrerequisites ? " --require-cloud-prerequisites" : ""
    }`,
    docs: currentDocs(repoRoot),
    versionSources: versionSources(repoRoot),
    gitStatus,
    dirtySummary,
    priorReceipts,
    phase5FinalSmoke: phase5FinalSmoke(priorReceipts),
    prereqSummary,
    malformedReceipts: badReceipts,
    contradictionFixture,
    receiptRootDiscovery: {
      method: receiptRootInfo.method,
      found_expected_file_count: receiptRootInfo.found_expected_file_count,
      root_label: receiptRootInfo.root === repoRoot ? "current_omo" : "historical_omo",
    },
    sourceReceipts: sourceReceipts(repoRoot, receiptRootInfo),
    privacy: { status: "PENDING" },
  });
  let markdown = buildMarkdown(report);
  const privacy = assertSanitized(report, markdown);
  if (privacy.status === "FAIL") {
    report = { ...report, status: "FAIL", privacy };
    markdown = buildMarkdown(report);
  } else {
    report = { ...report, privacy };
    markdown = buildMarkdown(report);
  }
  writeReportFiles(args.outDirAbs, reportJsonName, reportMdName, report, markdown);

  if (report.status === "FAIL") {
    const contradiction = contradictionFixture?.contradiction_name
      ? ` contradiction=${contradictionFixture.contradiction_name}`
      : "";
    console.error(`FAIL phase 6 preflight${contradiction}: ${args.outDir}/${reportJsonName}`);
    process.exit(2);
  }

  console.log(`${report.status} phase 6 cloud eval monitoring preflight: ${args.outDir}/${reportJsonName}`);
}
