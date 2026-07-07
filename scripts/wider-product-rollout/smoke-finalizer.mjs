#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { betaStatus, buildNegativeMatrix, claimsDefault, statusFromText } from "./smoke-negative-matrix.mjs";
import { removeEphemeralRuntimeTrees, sanitizeOutputTree, sanitizeText, scanOutputTree } from "./smoke-privacy-tree.mjs";
const [outDir, startedRaw, invocation, preflightRaw, manifestRaw, betaRaw, docsRaw, retentionRaw, runbookRaw, regressionRaw, killSwitchRaw, privacyRaw, canaryRaw, defaultBlockedRaw, missingArtifactsRaw] = process.argv.slice(2);

const startedMs = Number(startedRaw);
const statuses = { preflight: Number(preflightRaw), manifest: Number(manifestRaw), beta: Number(betaRaw), docs: Number(docsRaw), retention: Number(retentionRaw), runbook: Number(runbookRaw), regression: Number(regressionRaw), killSwitch: Number(killSwitchRaw), privacy: Number(privacyRaw) };
const negativeStatuses = { canary: Number(canaryRaw), defaultBlocked: Number(defaultBlockedRaw), missingArtifacts: Number(missingArtifactsRaw) };
const requiredArtifacts = [
  "summary.txt", "preflight-report.json", "manifest-validation.json", "beta-readiness.json", "docs-qa.md",
  "retention-report.json", "runbook-qa.md", "regression-gate.json", "kill-switches.json", "privacy-inspect.txt",
  "forbidden-tokens.txt", "negative-matrix.json", "deployment-action.txt", "cleanup-receipt.txt",
];
const forbiddenArtifact = /MORROW_PRIVACY_CANARY_RAW|PHASE6_RAW_CONTENT_CANARY_DO_NOT_STORE|\/Users\/|\/private\/|\/var\/folders\/|~\/\.codex|codex_access_token|CODEX_ACCESS_TOKEN|auth\.json|sk-[A-Za-z0-9]{12,}/;

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

function sanitizeArtifact(rel) {
  writeText(rel, sanitizeText(readText(rel)));
}

function copyArtifact(fromRel, toRel) {
  fs.copyFileSync(artifactPath(fromRel), artifactPath(toRel));
  sanitizeArtifact(toRel);
}

function allFreshAndClean(sanitizedFiles) {
  const rows = [];
  const recursiveFindings = scanOutputTree(outDir);
  if (recursiveFindings.length > 0) {
    throw new Error(`forbidden content leaked into output tree: ${recursiveFindings.join(", ")}`);
  }
  for (const rel of requiredArtifacts) {
    const file = artifactPath(rel);
    if (!fs.existsSync(file) || fs.statSync(file).size === 0) {
      throw new Error(`missing required artifact: ${rel}`);
    }
    const stat = fs.statSync(file);
    if (Number.isFinite(startedMs) && stat.mtimeMs + 1000 < startedMs) {
      throw new Error(`stale artifact: ${rel}`);
    }
    const text = fs.readFileSync(file, "utf8");
    if (rel !== "forbidden-tokens.txt" && forbiddenArtifact.test(text)) {
      throw new Error(`forbidden content leaked into artifact: ${rel}`);
    }
    if (rel.endsWith(".json")) JSON.parse(text);
    rows.push({ path: rel, bytes: stat.size, freshness: "current_run" });
  }
  const logsDir = artifactPath("command-logs");
  const logs = fs.readdirSync(logsDir).filter((name) => name.endsWith(".txt")).sort();
  if (logs.length === 0) throw new Error("command logs missing");
  for (const name of logs) {
    const text = fs.readFileSync(path.join(logsDir, name), "utf8");
    if (forbiddenArtifact.test(text)) throw new Error(`forbidden content leaked into command log: ${name}`);
  }
  return { rows, logs, recursiveScan: { sanitizedFiles, findings: 0 } };
}

function writeDeploymentAction() {
  writeText("deployment-action.txt", [
    "scenario: Phase 7 wider product rollout deployment action receipt",
    "deployment_action=none",
    "owner_approved_distribution_work: absent",
    "observable: runner executed local fixture and evidence gates only",
    "result: PASS",
    "",
  ].join("\n"));
}

function writeCleanupReceipt() {
  writeText("cleanup-receipt.txt", [
    "scenario: Phase 7 wider product rollout smoke cleanup",
    "observable: no live backend services, provider network paths, telemetry upload, remote config launch, artifact publish, real Messages read, Calendar mutation, or credential management was started",
    "process_policy: foreground local scripts only; command logs captured under command-logs/",
    "privacy_fixture_database: removed after scripts/privacy-inspect.sh completed",
    "result: PASS",
    "",
  ].join("\n"));
}

function writeNegativeMatrix(manifest, beta, regression) {
  const matrix = buildNegativeMatrix({ manifest, beta, regression, statuses, negativeStatuses });
  writeText("negative-matrix.json", `${JSON.stringify(matrix, null, 2)}\n`);
  return matrix;
}

function writeSummary({ manifest, beta, regression, matrix, artifacts, logs }) {
  const betaGate = betaStatus(beta);
  const blockedReason = betaGate === "BLOCKED"
    ? sanitizeText(`beta readiness BLOCKED: ${(beta.missing_prerequisites ?? ["missing signed/notarized beta artifacts"]).join("; ")}`)
    : "none";
  writeText("summary.txt", [
    "scenario: Phase 7 wider product rollout final smoke",
    `generated_at_utc: ${new Date().toISOString()}`,
    `invocation: ${invocation}`,
    "backend_observable: no live backend services, provider network paths, telemetry upload, real Messages read, Calendar mutation, remote config launch, artifact publish, or credential management used",
    `manifest_state: ${manifest.state}`,
    `default_availability_allowed: ${manifest.default_availability?.allowed === true}`,
    `beta_readiness: ${betaGate}`,
    `regression_gate: ${regression.overall_status}`,
    "deployment_action=none",
    `intentional_blocked_reason: ${blockedReason}`,
    `negative_matrix: ${matrix.result}`,
    "privacy_recursive_scan: PASS",
    `artifact_count: ${artifacts.length}`,
    `command_log_count: ${logs.length}`,
    "coverage: preflight",
    "coverage: manifest_validation",
    "coverage: beta_gate",
    "coverage: docs_qa",
    "coverage: retention_qa",
    "coverage: runbook_qa",
    "coverage: regression_gate",
    "coverage: kill_switch_check",
    "coverage: privacy_scan",
    "coverage: negative_fixtures",
    "coverage: artifact_freshness",
    "result: PASS",
    "",
  ].join("\n"));
}

function main() {
  copyArtifact("preflight/preflight-report.json", "preflight-report.json");
  copyArtifact("manifest/manifest-validation.json", "manifest-validation.json");
  copyArtifact("beta/beta-gate-result.json", "beta-readiness.json");
  copyArtifact("retention/retention-report.json", "retention-report.json");
  copyArtifact("regression/regression-gate.json", "regression-gate.json");
  copyArtifact("kill-switch/kill-switches.json", "kill-switches.json");
  writeDeploymentAction();
  writeCleanupReceipt();

  const manifestValidation = readJson("manifest-validation.json");
  const manifest = JSON.parse(fs.readFileSync(manifestValidation.manifest_path, "utf8"));
  const beta = readJson("beta-readiness.json");
  const regression = readJson("regression-gate.json");
  const killSwitches = readJson("kill-switches.json");
  const docsStatus = statusFromText(readText("docs-qa.md"));
  const runbookStatus = statusFromText(readText("runbook-qa.md"));
  const retention = readJson("retention-report.json");
  const privacyText = readText("privacy-inspect.txt");
  const betaGate = betaStatus(beta);
  removeEphemeralRuntimeTrees(outDir);
  const sanitizedFiles = sanitizeOutputTree(outDir);
  const privacyScan = scanOutputTree(outDir);
  if (privacyScan.length > 0) throw new Error(`forbidden content leaked into output tree: ${privacyScan.join(", ")}`);
  const matrix = writeNegativeMatrix(manifest, beta, regression);

  if (Object.values(statuses).some((status) => Number.isNaN(status))) throw new Error("invalid command status input");
  if (statuses.preflight !== 0 || statuses.manifest !== 0 || statuses.docs !== 0 || statuses.retention !== 0 || statuses.runbook !== 0 || statuses.killSwitch !== 0 || statuses.privacy !== 0) {
    throw new Error("one or more required non-release-blocked gates failed");
  }
  if (manifestValidation.status !== "PASS") throw new Error("manifest validation did not PASS");
  if (docsStatus !== "PASS") throw new Error("docs QA did not PASS");
  if (runbookStatus !== "PASS") throw new Error("runbook QA did not PASS");
  if (retention.status !== "PASS") throw new Error("retention QA did not PASS");
  if (killSwitches.status !== "PASS") throw new Error("kill-switch check did not PASS");
  if (!/privacy inspection passed/i.test(privacyText)) throw new Error("privacy-inspect did not report PASS");
  if (!["PASS", "BLOCKED"].includes(betaGate)) throw new Error("beta readiness was neither PASS nor BLOCKED");
  if (claimsDefault(manifest) && betaGate !== "PASS") throw new Error("default availability claimed while beta readiness is BLOCKED");
  if (!["PASS", "BLOCKED"].includes(regression.overall_status)) throw new Error("regression gate was neither PASS nor BLOCKED");
  if (regression.deployment_action !== "none") throw new Error("regression gate deployment_action was not none");
  if (claimsDefault(manifest) && regression.overall_status !== "PASS") throw new Error("default availability claimed without all prerequisites passing");
  if (!["beta", "blocked"].includes(manifest.state) && betaGate === "BLOCKED") throw new Error("blocked beta readiness is allowed only for beta or blocked manifest states");
  if (matrix.result !== "PASS") throw new Error("negative matrix did not PASS");

  const initialLogs = fs.readdirSync(artifactPath("command-logs")).filter((name) => name.endsWith(".txt")).sort();
  writeSummary({ manifest, beta, regression, matrix, artifacts: [], logs: initialLogs });
  const { rows, logs } = allFreshAndClean(sanitizedFiles);
  writeSummary({ manifest, beta, regression, matrix, artifacts: rows, logs });
  allFreshAndClean(sanitizedFiles);
}

try {
  main();
} catch (error) {
  console.error(`FAIL wider product rollout smoke finalizer: ${error instanceof Error ? error.message : String(error)}`);
  process.exitCode = 1;
}
