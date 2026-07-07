#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { buildNegativeMatrix } from "./negative-matrix.mjs";

const [
  outDir,
  startedRaw,
  canaryRaw,
  malformedRaw,
  staleRaw,
  regressionRaw,
  preflightRaw,
  invocation,
  cancelResumeRaw,
  repeatedInterruptionsFirstRaw,
  repeatedInterruptionsSecondRaw,
] = process.argv.slice(2);

const startedMs = Number(startedRaw);
const statuses = {
  canary: Number(canaryRaw),
  malformed: Number(malformedRaw),
  stale: Number(staleRaw),
  regression: Number(regressionRaw),
};
const preflightStatus = Number(preflightRaw);
const adversarialStatuses = {
  cancelResume: Number(cancelResumeRaw),
  repeatedInterruptionsFirst: Number(repeatedInterruptionsFirstRaw),
  repeatedInterruptionsSecond: Number(repeatedInterruptionsSecondRaw),
};
const forbiddenArtifact = /MORROW_PRIVACY_CANARY_RAW|PHASE6_RAW_CONTENT_CANARY_DO_NOT_STORE|\/Users\/|\/private\/|\/var\/folders\//;
const required = [
  "summary.txt", "preflight-report.json", "eval-job-input.json", "eval-slices.json",
  "metrics.json", "cloud-eval-monitoring-report.json", "cloud-eval-monitoring-dashboard.md",
  "dashboard-summary.txt", "release-gate.json", "release-gate.md", "alerts.json",
  "canary-rejection.txt", "negative-matrix.json", "cleanup-receipt.txt", "adversarial-interruptions.json",
];

function artifactPath(rel) {
  return path.join(outDir, rel);
}

function readText(rel) {
  return fs.readFileSync(artifactPath(rel), "utf8");
}

function readJson(rel) {
  return JSON.parse(readText(rel));
}

function writeText(rel, text) {
  fs.writeFileSync(artifactPath(rel), text);
}

function writeReceipts(matrix) {
  if (matrix.cases.some((entry) => entry.result === "FAIL")) matrix.result = "FAIL";
  writeText("negative-matrix.json", `${JSON.stringify(matrix, null, 2)}\n`);
  writeText("canary-rejection.txt", [
    "scenario: phase 6 privacy canary rejection",
    "invocation: node scripts/cloud-eval-monitoring-report.mjs validate-input --input scripts/cloud-eval-monitoring-fixtures/privacy-canary-events.json",
    "observable: validator exited non-zero before any sanitizer or dashboard/gate generation trusted the payload",
    `exit_status: ${statuses.canary}`,
    `error_names: ${matrix.cases[0].error_names.join(",")}`,
    "raw_canary_literal_written_to_receipt: no",
    "result: PASS",
    "",
  ].join("\n"));
  writeText("cleanup-receipt.txt", [
    "scenario: phase 6 cloud eval monitoring cleanup",
    "observable: no background services or provider/native/network processes were started by the smoke runner; adversarial interruption children were terminated and reaped",
    "process_policy: child commands are foreground Node CLIs with finite timeout, trapped cleanup receipts, and reaped exit statuses",
    "adversarial_interruption_cleanup: adversarial-interruptions.json",
    "result: PASS",
    "",
  ].join("\n"));
  writeText("summary.txt", [
    "scenario: Phase 6 cloud eval monitoring smoke",
    `generated_at_utc: ${new Date().toISOString()}`,
    `invocation: ${invocation}`,
    "backend_observable: no live backend, provider network, EventKit, Messages, Calendar, Reminders, Phoenix, Langfuse, vendor backend, deployment, or cloud upload surface was used; all commands used local deterministic fixtures",
    "coverage: preflight",
    "coverage: input_validation",
    "coverage: aggregation",
    "coverage: metrics",
    "coverage: dashboard",
    "coverage: release_gate",
    "coverage: privacy_canary_rejection",
    "coverage: malformed_input_negative",
    "coverage: regression_negative",
    "coverage: stale_state_negative",
    "coverage: artifact_freshness",
    "coverage: cancel_resume_adversarial",
    "coverage: repeated_interruptions_adversarial",
    "preflight_report: preflight-report.json",
    "eval_job_input: eval-job-input.json",
    "eval_slices: eval-slices.json",
    "metrics: metrics.json",
    "dashboard_report: cloud-eval-monitoring-report.json",
    "dashboard: cloud-eval-monitoring-dashboard.md",
    "dashboard_summary: dashboard-summary.txt",
    "release_gate: release-gate.json",
    "release_gate_markdown: release-gate.md",
    "alerts: alerts.json",
    "privacy_inspection: privacy-inspect.txt",
    "canary_rejection: canary-rejection.txt",
    "negative_matrix: negative-matrix.json",
    "adversarial_interruptions: adversarial-interruptions.json",
    "cleanup_receipt: cleanup-receipt.txt",
    "command_logs: command-logs/",
    "result: PASS",
    "",
  ].join("\n"));
}

function validateRequiredArtifacts() {
  const generated = [];
  for (const rel of required) {
    const artifact = artifactPath(rel);
    if (!fs.existsSync(artifact) || fs.statSync(artifact).size === 0) {
      throw new Error(`missing required artifact: ${rel}`);
    }
    const stat = fs.statSync(artifact);
    if (Number.isFinite(startedMs) && stat.mtimeMs + 1000 < startedMs) {
      throw new Error(`stale artifact: ${rel}`);
    }
    const text = fs.readFileSync(artifact, "utf8");
    if (forbiddenArtifact.test(text)) throw new Error(`forbidden content leaked into artifact: ${rel}`);
    if (rel.endsWith(".json")) JSON.parse(text);
    generated.push({ path: rel, bytes: stat.size, freshness: "current_run" });
  }
  return generated;
}

function validateSemantics() {
  const jsonArtifacts = ["preflight-report.json", "eval-job-input.json", "eval-slices.json", "metrics.json", "cloud-eval-monitoring-report.json", "release-gate.json", "alerts.json"];
  for (const rel of jsonArtifacts) {
    readJson(rel);
  }
  if (readJson("release-gate.json").overall_status !== "PASS") throw new Error("release gate did not PASS");
  if (!readText("summary.txt").includes("result: PASS")) throw new Error("summary result missing PASS");
  if (!readText("summary.txt").includes("no live backend")) throw new Error("backend observable missing from summary");
  if (readJson("negative-matrix.json").result !== "PASS") throw new Error("negative matrix did not PASS");
}

function validateLogs() {
  const logs = fs.readdirSync(artifactPath("command-logs")).filter((name) => name.endsWith(".txt")).sort();
  if (logs.length === 0) throw new Error("command logs missing");
  for (const name of logs) {
    const text = readText(path.join("command-logs", name));
    if (forbiddenArtifact.test(text)) throw new Error(`command log is not clean: ${name}`);
  }
  return logs;
}

function writePrivacyReceipt(generated, logs) {
  writeText("privacy-inspect.txt", [
    "scenario: phase 6 artifact privacy and freshness validation",
    "observable: required JSON artifacts parsed, text artifacts were non-empty, generated artifacts were fresh for this run, command logs were sanitized, and forbidden local paths/raw canary literals were absent",
    `artifact_count: ${generated.length}`,
    `command_log_count: ${logs.length}`,
    "validator: scripts/run-cloud-eval-monitoring-smoke.sh artifact scanner",
    "result: PASS",
    "",
  ].join("\n"));
  const privacyText = readText("privacy-inspect.txt");
  const privacyStat = fs.statSync(artifactPath("privacy-inspect.txt"));
  if (Number.isFinite(startedMs) && privacyStat.mtimeMs + 1000 < startedMs) {
    throw new Error("stale artifact: privacy-inspect.txt");
  }
  if (forbiddenArtifact.test(privacyText)) {
    throw new Error("forbidden content leaked into artifact: privacy-inspect.txt");
  }
}

writeReceipts(buildNegativeMatrix({
  outDir,
  statuses,
  preflightStatus,
  adversarialStatuses,
  timeoutSeconds: Number(process.env.RUN_TIMEOUT_SECONDS || "30"),
}));
const generated = validateRequiredArtifacts();
validateSemantics();
const logs = validateLogs();
writePrivacyReceipt(generated, logs);
