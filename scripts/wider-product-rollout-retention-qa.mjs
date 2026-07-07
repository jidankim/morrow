#!/usr/bin/env node

import fs from "node:fs";
import os from "node:os";
import path from "node:path";

import {
  parseArgs,
  planningEvidencePrefix,
  readText,
  runtimeDiagnosticsDirs,
  writeJson,
  writeText,
} from "./wider-product-rollout/retention-common.mjs";
import { addFixtureChecks, evaluateFixture } from "./wider-product-rollout/retention-fixture.mjs";
import { printSummary, renderMarkdown } from "./wider-product-rollout/retention-reporting.mjs";
import { createRuntimeTree, simulateDeleteAll } from "./wider-product-rollout/retention-runtime-tree.mjs";
import {
  addRetentionValueChecks,
  runDeleteAllScenario,
  runPlanningEvidenceScenario,
  runRetentionScenario,
} from "./wider-product-rollout/retention-scenarios.mjs";
import { addSourceChecks } from "./wider-product-rollout/retention-source-checks.mjs";

const repoRoot = process.cwd();

function loadSources() {
  return {
    appConfigText: readText(repoRoot, "src/domain/appConfig.ts"),
    settingsText: readText(repoRoot, "src/SettingsPrivacyControls.tsx"),
    privacyTestText: readText(repoRoot, "src/domain/appShell.persistencePrivacy.test.ts"),
    sinkText: readText(repoRoot, "crates/morrow-diagnostics/src/sink.rs"),
    deleteAllText: readText(repoRoot, "crates/morrow-storage/src/delete_all.rs"),
    protocolText: readText(repoRoot, "src-tauri/src/native_bridge/delete_all_protocol.rs"),
    docsText: readText(repoRoot, "docs/diagnostics-trace-eval.md"),
    nativeDeleteTestText: readText(repoRoot, "src-tauri/tests/native_delete/diagnostics.rs"),
  };
}

function writeFinalReport(args, outDir, checks, fixtureResult) {
  const status = checks.every((check) => check.status === "PASS") ? "PASS" : "FAIL";
  const report = {
    schema_version: "phase7_wider_product_rollout_retention_qa_v1",
    generated_at_utc: new Date().toISOString(),
    host: {
      platform: os.platform(),
      node: process.version,
    },
    out_dir: args.outDir,
    status,
    checks,
    runtime_diagnostics_dirs: runtimeDiagnosticsDirs,
    protected_planning_evidence_prefix: planningEvidencePrefix,
    fixture: fixtureResult,
  };
  writeJson(path.join(outDir, "retention-report.json"), report);
  writeText(path.join(outDir, "retention-report.md"), renderMarkdown(report));
  printSummary(checks, path.join(args.outDir, "retention-report.json"), status);
  process.exitCode = status === "PASS" ? 0 : 1;
}

function run(argv) {
  const args = parseArgs(argv);
  const outDir = path.resolve(repoRoot, args.outDir);
  fs.rmSync(outDir, { recursive: true, force: true });
  fs.mkdirSync(outDir, { recursive: true });

  const checks = [];
  addSourceChecks(checks, loadSources());
  addRetentionValueChecks(checks);

  const tree = createRuntimeTree(outDir);
  runRetentionScenario(checks, tree, outDir, args.outDir);
  const afterDelete = runDeleteAllScenario(checks, tree, simulateDeleteAll(tree), outDir, args.outDir);
  runPlanningEvidenceScenario(checks, tree, afterDelete, outDir, args.outDir);

  let fixtureResult;
  if (args.fixture !== undefined) {
    fixtureResult = evaluateFixture(repoRoot, args.fixture);
    addFixtureChecks(checks, fixtureResult, args.fixture);
  }
  writeFinalReport(args, outDir, checks, fixtureResult);
}

try {
  run(process.argv.slice(2));
} catch (error) {
  if (error instanceof Error) {
    console.error(error.message);
  } else {
    console.error("Unknown retention QA failure");
  }
  process.exitCode = 1;
}
