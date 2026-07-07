import { writeFile } from "node:fs/promises";
import path from "node:path";

import { suppressPassEvidenceOnBlockedRun } from "./beta-gate-checks.mjs";

export async function writeSummary(state, { outDir, runDir, bundleDir, branch, commit, dirty }) {
  const hasFail = state.checks.some((check) => check.status === "FAIL");
  const hasBlocked = state.checks.some((check) => check.status === "BLOCKED") || state.missingPrerequisites.size > 0;
  const overallStatus = hasBlocked ? "BLOCKED" : hasFail ? "FAIL" : "PASS";
  const completedAt = new Date();
  if (overallStatus !== "PASS") {
    await suppressPassEvidenceOnBlockedRun(state);
  }
  const missingPrerequisites = [...state.missingPrerequisites].sort();
  const summary = {
    task: "Todo 3 beta gate",
    run_id: path.basename(runDir),
    started_at: state.startedAt.toISOString(),
    completed_at: completedAt.toISOString(),
    branch,
    commit,
    cwd: state.repoRoot,
    bundle_dir: bundleDir,
    out_dir: outDir,
    current_run_dir: path.relative(state.repoRoot, runDir),
    dirty_paths: dirty,
    overall_status: overallStatus,
    missing_prerequisites: missingPrerequisites,
    checks: state.checks,
  };
  const jsonPath = path.join(outDir, "beta-gate-result.json");
  const mdPath = path.join(outDir, "beta-gate-result.md");
  await writeFile(jsonPath, `${JSON.stringify(summary, null, 2)}\n`, "utf8");
  await writeFile(mdPath, buildMarkdown(summary, missingPrerequisites), "utf8");
  console.log(`overall_status=${overallStatus}`);
  console.log(`evidence=${path.relative(state.repoRoot, jsonPath)}`);
  if (missingPrerequisites.length > 0) {
    console.log(`missing_prerequisites=${missingPrerequisites.join(", ")}`);
  }
  return { overallStatus, jsonPath };
}

function buildMarkdown(summary, missingPrerequisites) {
  return [
    "# Wider Product Rollout Beta Gate",
    "",
    `- overall_status=${summary.overall_status}`,
    `- run_id=${summary.run_id}`,
    `- started_at=${summary.started_at}`,
    `- completed_at=${summary.completed_at}`,
    `- branch=${summary.branch || "(unavailable)"}`,
    `- commit=${summary.commit || "(unavailable)"}`,
    `- bundle_dir=${summary.bundle_dir}`,
    `- current_run_dir=${summary.current_run_dir}`,
    "",
    "## Missing Prerequisites",
    "",
    ...(missingPrerequisites.length > 0 ? missingPrerequisites.map((item) => `- ${item}`) : ["- none"]),
    "",
    "## Checks",
    "",
    ...summary.checks.map((check) => `- ${check.status} ${check.name}: \`${check.command}\` -> ${check.evidence_path ?? check.details ?? "no evidence path"}`),
    "",
  ].join("\n");
}
