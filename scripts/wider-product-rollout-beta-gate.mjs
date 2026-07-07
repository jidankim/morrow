#!/usr/bin/env node
import { mkdir } from "node:fs/promises";
import path from "node:path";

import { discoverArtifacts } from "./wider-product-rollout/beta-gate-artifacts.mjs";
import {
  addMissing,
  createGateState,
  dirtyPaths,
  gitValue,
  runCheck,
  verifyEvidenceFile,
} from "./wider-product-rollout/beta-gate-checks.mjs";
import { exists, parseArgs, readIfPresent, resetOutputDirectory } from "./wider-product-rollout/beta-gate-io.mjs";
import { writeSummary } from "./wider-product-rollout/beta-gate-summary.mjs";

const startedAt = new Date();
const repoRoot = process.cwd();
const state = createGateState({ repoRoot, startedAt });

async function registerRequiredScripts() {
  for (const script of [
    "scripts/release-config-qa.mjs",
    "scripts/release-artifact-qa.mjs",
    "scripts/release-signing-qa.sh",
    "scripts/beta-release-docs-qa.mjs",
    "scripts/release-dmg-install-qa.sh",
    "scripts/release-packaged-app-launch-qa.sh",
    "scripts/release-manifest-qa.mjs",
  ]) {
    if (!((await exists(path.join(repoRoot, script)))?.isFile())) {
      addMissing(state, script);
      state.checks.push({
        name: `required script exists: ${script}`,
        status: "BLOCKED",
        command: `stat ${script}`,
        exit_code: 1,
        timed_out: false,
        stdout_path: null,
        evidence_path: null,
        details: `${script} is missing`,
      });
    }
  }
}

async function registerPlanGuardrails() {
  const planPath = ".omo/plans/beta-release-distribution.md";
  const planText = await readIfPresent(path.join(repoRoot, planPath));
  const hasGuardrails = /unsigned artifacts are diagnostic-only/i.test(planText) && /signed\/notarized/i.test(planText);
  state.checks.push({
    name: "beta release-distribution plan guardrails are readable",
    status: hasGuardrails ? "PASS" : "BLOCKED",
    command: `read ${planPath}`,
    exit_code: hasGuardrails ? 0 : 1,
    timed_out: false,
    stdout_path: null,
    evidence_path: planPath,
    details: hasGuardrails
      ? "signed/notarized beta and diagnostic-only unsigned guardrails present"
      : "required signed/notarized diagnostic-only guardrails were not found",
  });
  if (!hasGuardrails) {
    addMissing(state, "beta release-distribution plan guardrails");
  }
}

async function runGateChecks(bundleDir, runDir) {
  await runCheck(state, {
    name: "release config QA",
    command: "node",
    args: ["scripts/release-config-qa.mjs", "src-tauri/tauri.conf.json", "package.json", path.join(runDir, "release-config-qa.md")],
    evidencePath: path.join(runDir, "release-config-qa.md"),
  });
  await runCheck(state, {
    name: "release artifact QA",
    command: "node",
    args: ["scripts/release-artifact-qa.mjs", bundleDir, path.join(runDir, "release-artifact-qa.md")],
    evidencePath: path.join(runDir, "release-artifact-qa.md"),
  });
  await runCheck(state, {
    name: "release signing and notarization QA",
    command: "scripts/release-signing-qa.sh",
    args: [bundleDir, path.join(runDir, "release-signing-qa.md")],
    evidencePath: path.join(runDir, "release-signing-qa.md"),
  });
  await runCheck(state, {
    name: "beta tester docs QA",
    command: "node",
    args: ["scripts/beta-release-docs-qa.mjs", "README.md", "docs/beta-testing.md", path.join(runDir, "beta-release-docs-qa.md")],
    evidencePath: path.join(runDir, "beta-release-docs-qa.md"),
  });
}

async function verifyReadinessEvidence() {
  await verifyEvidenceFile(state, {
    name: "Codex readiness/redaction evidence",
    filePath: path.join(repoRoot, ".omo/evidence/task-5-beta-release-distribution.md"),
    requiredPatterns: [/Native Codex auth QA/i, /Provider UI QA/i, /test result|PASS|ok/i],
    forbiddenPatterns: [/codex_access_token/i, /sk-[A-Za-z0-9]/, /auth\.json/i, /codex login status --raw/i],
  });
  await verifyEvidenceFile(state, {
    name: "beta build transcript",
    filePath: path.join(repoRoot, ".omo/evidence/task-6-beta-release-distribution.md"),
    requiredPatterns: [/npm run tauri:build:beta|tauri build --bundles app,dmg/i],
    forbiddenPatterns: [/tauri:build:diagnostic|diagnostic artifact/i, /RESULT:\s*PASS[\s\S]*unsigned/i],
  });
}

async function registerDiagnosticArtifacts(bundleDir) {
  const artifacts = await discoverArtifacts(bundleDir);
  if (artifacts.diagnostic.length === 0) {
    return;
  }
  state.checks.push({
    name: "diagnostic artifacts are not beta-ready",
    status: "BLOCKED",
    command: `inspect ${bundleDir}`,
    exit_code: 1,
    timed_out: false,
    stdout_path: null,
    evidence_path: null,
    details: artifacts.diagnostic.join(", "),
  });
  addMissing(state, "non-diagnostic signed/notarized beta artifacts");
}

async function runInstallLaunchAndManifestChecks(bundleDir, runDir) {
  await runCheck(state, {
    name: "DMG install QA",
    command: "scripts/release-dmg-install-qa.sh",
    args: [bundleDir, path.join(runDir, "release-dmg-install-qa.md")],
    evidencePath: path.join(runDir, "release-dmg-install-qa.md"),
    timeoutMs: 180_000,
  });
  await runCheck(state, {
    name: "packaged app launch screenshot QA",
    command: "scripts/release-packaged-app-launch-qa.sh",
    args: [bundleDir, path.join(runDir, "release-packaged-app-launch-qa.md")],
    evidencePath: path.join(runDir, "release-packaged-app-launch-qa.md"),
    timeoutMs: 180_000,
  });
  await runCheck(state, {
    name: "release manifest/checklist QA",
    command: "node",
    args: ["scripts/release-manifest-qa.mjs", "docs/beta-release-checklist.md", bundleDir, path.join(runDir, "release-manifest-qa.md")],
    evidencePath: path.join(runDir, "release-manifest-qa.md"),
  });
}

async function main() {
  const { bundleDir, outDir } = parseArgs(process.argv.slice(2));
  const absoluteOutDir = path.resolve(repoRoot, outDir);
  await resetOutputDirectory(repoRoot, absoluteOutDir, bundleDir);
  const runId = `run-${startedAt.toISOString().replace(/[:.]/g, "-")}-${process.pid}`;
  const runDir = path.join(absoluteOutDir, runId);
  await mkdir(runDir, { recursive: true });

  const branch = await gitValue(repoRoot, ["rev-parse", "--abbrev-ref", "HEAD"]);
  const commit = await gitValue(repoRoot, ["rev-parse", "HEAD"]);
  const dirty = await dirtyPaths(repoRoot);

  await registerRequiredScripts();
  await registerPlanGuardrails();
  await runGateChecks(bundleDir, runDir);
  await verifyReadinessEvidence();
  await registerDiagnosticArtifacts(bundleDir);
  await runInstallLaunchAndManifestChecks(bundleDir, runDir);

  const { overallStatus } = await writeSummary(state, {
    outDir: absoluteOutDir,
    runDir,
    bundleDir,
    branch,
    commit,
    dirty,
  });
  process.exitCode = overallStatus === "PASS" ? 0 : 1;
}

main().catch((error) => {
  console.error(error.message);
  process.exitCode = 2;
});
