import fs from "node:fs";
import path from "node:path";
import process from "node:process";

import { REQUIRED_SOURCES } from "./preflight-constants.mjs";
import {
  betaPrerequisiteStatus,
  docsBoundaryChecks,
  parseFixture,
  phase6Status,
  privacyDefaults
} from "./preflight-checks.mjs";
import { buildDirtySummary } from "./preflight-dirty.mjs";
import { git, readJson, readText, sourceReceipt } from "./preflight-io.mjs";

export function buildReport(repoRoot, args) {
  const generatedAt = new Date().toISOString();
  const runId = `phase7-preflight-${generatedAt.replace(/[^0-9TZ]/g, "")}-${process.pid}`;
  const gitStatus = git(["status", "--porcelain=v1", "--untracked-files=all"]);
  const rolloutPlanText = readText(repoRoot, ".omo/plans/wider-product-rollout.md");
  const draftText = readText(repoRoot, ".omo/drafts/wider-product-rollout.md");
  const betaPlanText = readText(repoRoot, ".omo/plans/beta-release-distribution.md");
  const packageJson = readJson(repoRoot, "package.json");
  const tauriConfig = readJson(repoRoot, "src-tauri/tauri.conf.json");
  const appConfigText = readText(repoRoot, "src/domain/appConfig.ts");
  const readmeText = readText(repoRoot, "README.md");
  const axiText = readText(repoRoot, "docs/axi-phase-0-feedback-eval-baseline.md");
  const fixture = parseFixture(repoRoot, args.fixture);
  const privacyConfig = privacyDefaults(appConfigText);
  const docsChecks = docsBoundaryChecks(readmeText, axiText, rolloutPlanText, draftText);
  const phase6 = phase6Status(repoRoot);
  const beta = betaPrerequisiteStatus(betaPlanText);
  const status =
    fixture.status === "FAIL" || privacyConfig.status === "FAIL" || docsChecks.status === "FAIL" ? "FAIL" : "PASS";
  return {
    schema_version: "phase7_wider_product_rollout_preflight_v1",
    current_run_marker: true,
    run_id: runId,
    generated_at_utc: generatedAt,
    status,
    phase: 7,
    scope: "wider_product_rollout",
    rollout_source_of_truth: "static_local_manifest",
    remote_config: "out_of_scope",
    live_remote_config_allowed: false,
    telemetry_upload_allowed: false,
    beta_plan_source_of_truth: ".omo/plans/beta-release-distribution.md",
    docs_mirror_source_of_truth: false,
    invocation: `node scripts/wider-product-rollout-preflight.mjs --out-dir ${args.outDir}${
      args.fixture ? ` --fixture ${args.fixture}` : ""
    }`,
    package_app_versions: {
      package: {
        name: packageJson.name,
        version: packageJson.version
      },
      app: {
        product_name: tauriConfig.productName,
        version: tauriConfig.version,
        identifier_present: typeof tauriConfig.identifier === "string" && tauriConfig.identifier.length > 0
      }
    },
    source_receipts: REQUIRED_SOURCES.map((relativePath) => sourceReceipt(repoRoot, relativePath)),
    docs_boundary_checks: docsChecks,
    phase6_fixture_monitoring: phase6,
    beta_prerequisites: beta,
    privacy_config_defaults: privacyConfig,
    dirty_worktree_porcelain_v1_untracked_all: gitStatus,
    dirty_worktree_classification: buildDirtySummary(gitStatus),
    product_vs_evidence_retention_boundary: {
      status: "RECORDED",
      product_runtime_artifacts:
        "Morrow-owned runtime data and local diagnostics are product artifacts governed by runtime privacy/delete behavior.",
      planning_evidence_artifacts:
        ".omo/evidence receipts are planning artifacts retained as verification records and are not product runtime storage.",
      delete_all_boundary:
        "This Todo 1 receipt records the boundary only; it does not change product deletion behavior."
    },
    hero_asset_classification: {
      path: "public/assets/morrow-hero.png",
      exists: fs.existsSync(path.join(repoRoot, "public/assets/morrow-hero.png")),
      classification: "unrelated_unless_later_phase7_receipt_intentionally_uses_it",
      phase7_intentional_use: false
    },
    contradiction_fixture: fixture,
    forbidden_raw_content: { status: "PENDING", checked_categories: [], hits: [] }
  };
}
