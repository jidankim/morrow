import { readFile } from "node:fs/promises";
import { execFile } from "node:child_process";
import path from "node:path";
import { promisify } from "node:util";
import { buildInvocation, readJson, writeReport } from "./report-io.mjs";
import { validateManifestObject } from "./report-validation.mjs";

const execFileAsync = promisify(execFile);
const MIN_FRESH_DATE_UTC = Date.parse("2026-07-07T00:00:00Z");

const REQUIRED_BUDGETS = [
  "privacy_canary_result",
  "malformed_input_negative",
  "stale_state_negative",
  "regression_gate_negative",
  "release_gate_status",
  "latency_error_regression",
  "drift",
  "false_positive_clusters",
  "precision_recall",
  "approval_rejection_edit_rates",
  "beta_package_qa",
  "support_readiness",
];

function add(checks, name, status, details = "") {
  checks.push({ name, status, details });
}

async function readJsonArtifact(checks, filePath, name) {
  try {
    const value = await readJson(filePath);
    add(checks, `${name} parses`, "PASS", filePath);
    return value;
  } catch (error) {
    add(checks, `${name} parses`, "FAIL", `${filePath}: ${error.message}`);
    return null;
  }
}

function checkFreshArtifact(checks, artifact, name) {
  const generatedAt = artifact?.generated_at_utc ?? artifact?.completed_at ?? artifact?.validated_at ?? null;
  if (typeof generatedAt !== "string") {
    add(checks, `${name} freshness is covered by required JSON presence`, "PASS", "no timestamp field in this artifact schema");
    return;
  }

  const timestamp = Date.parse(generatedAt);
  add(
    checks,
    `${name} is fresh for Phase 7 Todo 7`,
    Number.isFinite(timestamp) && timestamp >= MIN_FRESH_DATE_UTC ? "PASS" : "FAIL",
    `observed ${JSON.stringify(generatedAt)}`,
  );
}

async function dirtyWorktreeStatus() {
  try {
    const { stdout } = await execFileAsync("git", ["status", "--short", "--untracked-files=all"], {
      cwd: process.cwd(),
      maxBuffer: 1024 * 1024,
    });
    return stdout.trim().split("\n").filter(Boolean);
  } catch (error) {
    return [`git status unavailable: ${error.message}`];
  }
}

async function readBetaArtifact(checks, filePath) {
  try {
    const value = await readJson(filePath);
    add(checks, "beta readiness parses", "PASS", filePath);
    return value;
  } catch (error) {
    add(checks, "beta readiness is present or explicitly blocked", "BLOCKED", `${filePath}: ${error.message}`);
    return null;
  }
}

async function readTextArtifact(checks, filePath, name) {
  try {
    const value = await readFile(filePath, "utf8");
    add(checks, `${name} is readable`, value.length > 0 ? "PASS" : "FAIL", filePath);
    return value;
  } catch (error) {
    add(checks, `${name} is readable`, "FAIL", `${filePath}: ${error.message}`);
    return "";
  }
}

function casePassed(matrix, token) {
  return matrix?.cases?.some((entry) => entry.name?.includes(token) && entry.result === "PASS") === true;
}

function minPrecisionRecall(metrics) {
  const rows = metrics?.metric_families?.precision_recall_by_intent_type ?? [];
  const values = rows.flatMap((row) => [row.precision?.value, row.recall?.value]).filter((value) => typeof value === "number");
  return values.length === 0 ? null : Math.min(...values);
}

function maxDrift(metrics) {
  const rows = metrics?.metric_families?.drift_by_release ?? [];
  const values = rows.map((row) => row.drift_score).filter((value) => typeof value === "number");
  return values.length === 0 ? null : Math.max(...values);
}

function latencyAndErrorPass(metrics, budget) {
  const regressions = metrics?.metric_families?.regressions_against_baseline_release ?? {};
  const latency = regressions.router_latency_p95_ms ?? [];
  const errors = regressions.provider_error_rate ?? [];
  const maxDelta = budget?.max_latency_p95_delta_ms ?? 5000;
  const maxErrorDelta = budget?.max_error_rate_delta ?? 0.05;
  return latency.every((row) => row.status !== "computed" || row.delta <= maxDelta) &&
    errors.every((row) => row.status !== "computed" || row.delta <= maxErrorDelta);
}

function betaStatus(beta) {
  if (!beta) return { status: "BLOCKED", missing: ["beta readiness output"] };
  if (beta.overall_status === "PASS") return { status: "PASS", missing: [] };
  if (beta.overall_status === "BLOCKED") return { status: "BLOCKED", missing: beta.missing_prerequisites ?? [] };
  return { status: "FAIL", missing: [] };
}

function summarizeMissing(values) {
  return values.map((entry) => String(entry).replace(process.cwd(), "<repo>").replace(/\/Users\/[^/\s]+/g, "<private-home>"));
}

export async function runGate(args) {
  if (!args.manifest || !args.phase6_dir || !args.beta_dir || !args.out_dir) {
    throw new Error("Usage: gate --manifest <path> --phase6-dir <dir> --beta-dir <dir> --out-dir <dir>");
  }

  const checks = [];
  const manifest = await readJsonArtifact(checks, args.manifest, "manifest");
  const phase6Release = await readJsonArtifact(checks, path.join(args.phase6_dir, "release-gate.json"), "Phase 6 release gate");
  const phase6Matrix = await readJsonArtifact(checks, path.join(args.phase6_dir, "negative-matrix.json"), "Phase 6 negative matrix");
  const phase6Metrics = await readJsonArtifact(checks, path.join(args.phase6_dir, "metrics.json"), "Phase 6 metrics");
  await readTextArtifact(checks, path.join(args.phase6_dir, "summary.txt"), "Phase 6 summary");
  const betaReport = await readBetaArtifact(checks, path.join(args.beta_dir, "beta-gate-result.json"));
  const retention = await readJsonArtifact(checks, args.retention_dir ? path.join(args.retention_dir, "retention-report.json") : ".omo/evidence/phase-7-wider-product-rollout/task-5/retention/retention-report.json", "retention QA");
  const docsQa = await readTextArtifact(checks, args.docs_qa ?? ".omo/evidence/phase-7-wider-product-rollout/task-4/docs-qa.md", "docs QA");
  const runbookQa = await readTextArtifact(checks, args.runbook_qa ?? ".omo/evidence/phase-7-wider-product-rollout/task-6/runbook-qa.md", "runbook QA");
  const dirtyPaths = await dirtyWorktreeStatus();

  checkFreshArtifact(checks, phase6Release, "Phase 6 release gate");
  checkFreshArtifact(checks, phase6Matrix, "Phase 6 negative matrix");
  checkFreshArtifact(checks, phase6Metrics, "Phase 6 metrics");
  checkFreshArtifact(checks, betaReport, "beta readiness");
  checkFreshArtifact(checks, retention, "retention QA");

  const validation = manifest ? validateManifestObject(manifest) : { checks: [] };
  for (const entry of validation.checks) add(checks, `manifest validation: ${entry.name}`, entry.status, entry.details);
  const budgets = manifest?.regression_budgets ?? {};
  for (const key of REQUIRED_BUDGETS) add(checks, `regression budget ${key} is declared`, budgets[key] ? "PASS" : "FAIL");
  add(
    checks,
    "Phase 6 is fixture-monitoring evidence only",
    manifest?.gates?.phase_6_monitoring_role === "fixture_monitoring_input_only" &&
      phase6Release?.live_cloud_prerequisites === "not_required_for_fixture_gate"
      ? "PASS"
      : "FAIL",
    "deployment claims are not accepted",
  );
  add(checks, "Phase 6 release gate passed with no deployment action", phase6Release?.overall_status === "PASS" && phase6Release?.deployment_action === "none" ? "PASS" : "FAIL");
  add(checks, "privacy canary negative case passed", casePassed(phase6Matrix, "privacy") ? "PASS" : "FAIL");
  add(checks, "malformed input negative case passed", casePassed(phase6Matrix, "malformed_input") ? "PASS" : "FAIL");
  add(checks, "stale state negative case passed", casePassed(phase6Matrix, "stale_state") ? "PASS" : "FAIL");
  add(checks, "regression gate negative case passed", casePassed(phase6Matrix, "regression_gate") ? "PASS" : "FAIL");
  add(checks, "latency and error regression budgets pass", latencyAndErrorPass(phase6Metrics, budgets.latency_error_regression) ? "PASS" : "FAIL");
  add(checks, "drift budget passes", maxDrift(phase6Metrics) <= (budgets.drift?.max_drift_score ?? 0.75) ? "PASS" : "FAIL");
  add(checks, "false-positive cluster budget passes", (phase6Metrics?.metric_families?.false_positive_clusters?.length ?? 0) <= (budgets.false_positive_clusters?.max_count ?? 3) ? "PASS" : "FAIL");
  add(checks, "precision/recall budget passes where available", minPrecisionRecall(phase6Metrics) >= (budgets.precision_recall?.minimum ?? 0.9) ? "PASS" : "FAIL");
  add(checks, "approval/rejection/edit rates are available", Boolean(phase6Metrics?.metric_families?.approval_rejection_edit_rates?.overall) ? "PASS" : "FAIL");
  add(checks, "retention QA passed", retention?.status === "PASS" ? "PASS" : "FAIL");
  add(checks, "docs QA passed", docsQa.includes("Result: PASS") ? "PASS" : "FAIL");
  add(checks, "runbook QA and support readiness passed", runbookQa.includes("Result: PASS") && runbookQa.includes("Support Escalation: PASS") ? "PASS" : "FAIL");

  const betaGate = betaStatus(betaReport);
  add(checks, "beta package QA passed or is explicitly blocked", betaGate.status === "FAIL" ? "FAIL" : betaGate.status);
  const claimsDefault = manifest?.state === "default_available" || manifest?.default_availability?.allowed === true;
  add(
    checks,
    "default availability is not claimed while beta prereqs are blocked",
    claimsDefault && betaGate.status !== "PASS" ? "FAIL" : "PASS",
    `manifest_state=${JSON.stringify(manifest?.state)} beta_status=${betaGate.status}`,
  );
  add(
    checks,
    "dirty worktree cannot authorize default availability",
    claimsDefault && dirtyPaths.length > 0 ? "FAIL" : "PASS",
    dirtyPaths.length === 0 ? "clean" : `${dirtyPaths.length} dirty path(s) recorded`,
  );
  const hardFail = checks.some((entry) => entry.status === "FAIL");
  const overallStatus = hardFail ? "FAIL" : betaGate.status === "BLOCKED" ? "BLOCKED" : "PASS";
  const report = {
    mode: "gate",
    overall_status: overallStatus,
    validated_at: new Date().toISOString(),
    invocation: buildInvocation(),
    deployment_action: "none",
    phase6_role: "fixture_monitoring_input_only",
    dirty_worktree: {
      status: dirtyPaths.length === 0 ? "clean" : "dirty",
      path_count: dirtyPaths.length,
      policy: "dirty worktree cannot authorize default availability",
    },
    beta_missing_prerequisites: summarizeMissing(betaGate.missing),
    regression_budget_observations: {
      min_precision_recall: minPrecisionRecall(phase6Metrics),
      max_drift_score: maxDrift(phase6Metrics),
      false_positive_cluster_count: phase6Metrics?.metric_families?.false_positive_clusters?.length ?? null,
      approval_rejection_edit_rates: phase6Metrics?.metric_families?.approval_rejection_edit_rates?.overall ?? null,
    },
    checks,
  };
  const reportPath = await writeReport(args.out_dir, "regression-gate.json", report);
  console.log(`${overallStatus} rollout regression gate: ${reportPath}`);
  if (overallStatus === "FAIL") process.exitCode = 1;
}
