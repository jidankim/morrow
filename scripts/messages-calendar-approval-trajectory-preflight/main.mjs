import process from "node:process";
import path from "node:path";

import { parseArgs, reportJsonName, reportMdName } from "./cli.mjs";
import {
  dirtyWorktreeAllowlist,
  liveGateClassification,
  overallStatus,
  providerResolution,
  validateProviderConflictFixture,
} from "./gates.mjs";
import { readText, runGit, writeReportFiles } from "./io.mjs";
import { buildMarkdown } from "./markdown.mjs";
import * as parsers from "./parsers.mjs";
import {
  boulderSnapshots,
  buildReport,
  evidenceReceipts,
  predecessorPlanState,
  sourceReceipts,
} from "./report.mjs";
import { discoverHistoricalRoot, providerAuthDocs, sourceLabel } from "./sources.mjs";

export function main() {
  const repoRoot = process.cwd();
  const args = parseArgs(process.argv.slice(2), repoRoot);
  const generatedAt = new Date().toISOString();
  const runId = `phase5-preflight-${generatedAt.replace(/[^0-9TZ]/g, "")}-${process.pid}`;
  const gitStatus = args.fixtureGitStatus
    ? readText(path.resolve(repoRoot, args.fixtureGitStatus))
    : runGit(["status", "--porcelain=v1", "--untracked-files=all"]);
  const historicalRootInfo = discoverHistoricalRoot(repoRoot);
  const historicalLabel = sourceLabel(historicalRootInfo, repoRoot);
  const receipts = evidenceReceipts({ historicalLabel, historicalRootInfo, parsers });
  const docs = providerAuthDocs(repoRoot, historicalRootInfo);
  const resolution = providerResolution(docs);
  const dirtyAllowlist = dirtyWorktreeAllowlist(gitStatus);
  const liveGate = liveGateClassification({
    messagesCalendarTask7: receipts.task7,
    providerResolution: resolution,
    dirtyAllowlist,
  });
  const fixtureResult = validateProviderConflictFixture(args.fixtureProviderConflict);
  const fixtureFailed = fixtureResult?.status === "PROVIDER_AUTH_CONTRADICTION";
  const status = overallStatus({ fixtureFailed, dirtyAllowlist });

  const report = buildReport({
    repoRoot,
    runId,
    generatedAt,
    gitStatus,
    receipts,
    docs,
    resolution,
    dirtyAllowlist,
    liveGate,
    fixtureResult,
    overallStatus: status,
    boulderSnapshot: boulderSnapshots(repoRoot, historicalLabel, historicalRootInfo),
    predecessorPlanState: predecessorPlanState(repoRoot, historicalLabel, historicalRootInfo),
    sourceReceipts: sourceReceipts({ repoRoot, historicalRootInfo, historicalLabel }),
    historicalDiscovery: {
      method: historicalRootInfo.method,
      found_expected_file_count: historicalRootInfo.found_expected_file_count,
      root_label: historicalLabel,
    },
  });
  writeReportFiles(args.outDirAbs, reportJsonName, reportMdName, report, buildMarkdown(report));

  if (fixtureFailed) {
    console.log("FAIL phase 5 preflight provider-auth contradiction");
    console.error(
      "provider-auth contradiction: README/Codex CLI source conflicts with MORROW_REAL_QA_OPENAI_API_KEY fixture and no explicit reconciliation note was supplied",
    );
    process.exit(2);
  }

  console.log(`${status} phase 5 preflight source-of-truth gate: ${args.outDir}/${reportJsonName}`);
}
