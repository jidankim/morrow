import fs from "node:fs";
import path from "node:path";

import { FORBIDDEN_CONTENT_PATTERNS, PHASE6_DIR } from "./preflight-constants.mjs";
import { readJson, readText, sourceReceipt } from "./preflight-io.mjs";

function parseTodoStatus(planText) {
  const todos = [];
  const todoPattern = /^- \[(?<mark>[ xX])\] (?<number>\d+)\. (?<title>.+)$/gm;
  for (const match of planText.matchAll(todoPattern)) {
    todos.push({
      number: Number(match.groups.number),
      title: match.groups.title.trim(),
      checked: match.groups.mark.toLowerCase() === "x"
    });
  }
  return todos;
}

export function betaPrerequisiteStatus(betaPlanText) {
  const todos = parseTodoStatus(betaPlanText);
  const requiredTodoNumbers = [6, 7, 8, 9];
  const required = todos.filter((todo) => requiredTodoNumbers.includes(todo.number));
  const missing = required.filter((todo) => !todo.checked);
  return {
    status: missing.length === 0 ? "PASS" : "BLOCKED",
    source_of_truth: ".omo/plans/beta-release-distribution.md",
    docs_mirror_policy: "docs/plans/beta-release-distribution/* is a mirror unless a later receipt proves otherwise.",
    required_beta_closure_todos: required,
    missing_prerequisites: missing.map((todo) => ({
      todo: todo.number,
      title: todo.title,
      reason: "unchecked in canonical beta release-distribution plan"
    })),
    default_availability_allowed: missing.length === 0
  };
}

export function phase6Status(repoRoot) {
  const summaryPath = `${PHASE6_DIR}/summary.txt`;
  const releaseGatePath = `${PHASE6_DIR}/release-gate.json`;
  const negativeMatrixPath = `${PHASE6_DIR}/negative-matrix.json`;
  const summary = fs.existsSync(path.join(repoRoot, summaryPath)) ? readText(repoRoot, summaryPath) : "";
  const releaseGate = fs.existsSync(path.join(repoRoot, releaseGatePath)) ? readJson(repoRoot, releaseGatePath) : {};
  const negativeMatrix = fs.existsSync(path.join(repoRoot, negativeMatrixPath))
    ? readJson(repoRoot, negativeMatrixPath)
    : {};
  const expectedFiles = [
    "summary.txt",
    "preflight-report.json",
    "eval-job-input.json",
    "eval-slices.json",
    "metrics.json",
    "cloud-eval-monitoring-report.json",
    "cloud-eval-monitoring-dashboard.md",
    "dashboard-summary.txt",
    "release-gate.json",
    "release-gate.md",
    "alerts.json",
    "privacy-inspect.txt",
    "canary-rejection.txt",
    "negative-matrix.json",
    "cleanup-receipt.txt"
  ];
  const fileReceipts = expectedFiles.map((name) => sourceReceipt(repoRoot, `${PHASE6_DIR}/${name}`));
  return {
    status: "FIXTURE_MONITORING_ONLY",
    source_dir: PHASE6_DIR,
    summary_result_pass: /^result: PASS$/m.test(summary),
    backend_observable_no_live_surface: /no live backend[\s\S]+deployment, or cloud upload surface was used/.test(summary),
    release_gate_overall_status: releaseGate.overall_status ?? "UNKNOWN",
    release_gate_deployment_action: releaseGate.deployment_action ?? "unknown",
    negative_matrix_result: negativeMatrix.result ?? "UNKNOWN",
    deployed_rollout_evidence: false,
    default_availability_evidence: false,
    file_receipts: fileReceipts
  };
}

export function privacyDefaults(appConfigText) {
  const telemetryTypeFalse = /telemetryEnabled:\s*false/.test(appConfigText);
  const crashTypeFalse = /crashLogExcerptsEnabled:\s*false/.test(appConfigText);
  const telemetrySchemaFalse = /telemetryEnabled:\s*z\.literal\(false\)\.default\(false\)/.test(appConfigText);
  const crashSchemaFalse = /crashLogExcerptsEnabled:\s*z\.literal\(false\)\.default\(false\)/.test(appConfigText);
  const telemetryDefaultFalse = /telemetryEnabled:\s*false/.test(appConfigText);
  const localDiagnosticsDefaultFalse = /localDiagnosticsEnabled:\s*false/.test(appConfigText);
  const retentionDefault30 = /localDiagnosticsRetentionDays:\s*30/.test(appConfigText);
  const retentionBounds = /localDiagnosticsRetentionDays:[\s\S]+\.min\(1\)\.max\(365\)\.default\(30\)/.test(appConfigText);
  return {
    status:
      telemetryTypeFalse &&
      crashTypeFalse &&
      telemetrySchemaFalse &&
      crashSchemaFalse &&
      telemetryDefaultFalse &&
      localDiagnosticsDefaultFalse &&
      retentionDefault30 &&
      retentionBounds
        ? "PASS"
        : "FAIL",
    telemetry_hard_off: telemetryTypeFalse && telemetrySchemaFalse && telemetryDefaultFalse,
    crash_log_excerpts_hard_off: crashTypeFalse && crashSchemaFalse,
    local_diagnostics_default_enabled: false,
    local_diagnostics_retention_days_default: retentionDefault30 ? 30 : "UNKNOWN",
    retention_days_bounds: retentionBounds ? "1..365" : "UNKNOWN"
  };
}

export function docsBoundaryChecks(readmeText, axiText, rolloutPlanText, draftText) {
  const docsText = [readmeText, axiText, rolloutPlanText, draftText].join("\n");
  const phase6FixtureOnly =
    /Phase 6[\s\S]{0,220}(fixture-backed|fixture-monitoring|fixture input|local smoke artifacts)/i.test(docsText) &&
    /Phase 6[\s\S]{0,220}(not deployed rollout evidence|separate from deployed rollout|does not prove deployed rollout)/i.test(
      docsText
    );
  const deployedRolloutFuture =
    /(deployed rollout|live cloud rollout)[\s\S]{0,160}(future work|out of scope|not live deployment|not default availability)/i.test(
      docsText
    ) ||
    /(not live deployment|does not prove deployed rollout)/i.test(docsText);
  const syntheticNotLiveCalendarProof =
    /(synthetic|packaging|Phase 6 dashboards?)[\s\S]{0,220}(do not treat|does not prove|No claim)[\s\S]{0,220}(real message|real Calendar|live Messages-to-Calendar|full live Messages-to-Calendar)/i.test(
      docsText
    ) ||
    /(do not treat|does not prove|No claim)[\s\S]{0,220}(synthetic|packaging|Phase 6 dashboards?)[\s\S]{0,220}(real message|real Calendar|live Messages-to-Calendar|full live Messages-to-Calendar)/i.test(
      docsText
    );
  const defaultAvailabilityGated =
    /default availability[\s\S]{0,220}(require|requires|blocked|allowed only|not claimed|false|all gates pass|signed and notarized)/i.test(
      docsText
    );
  const liveCloudFuture =
    /(live cloud|cloud upload|live backend)[\s\S]{0,180}(future work|out of scope|not|no live)/i.test(docsText) ||
    /no live backend[\s\S]{0,180}(deployment|cloud upload)/i.test(docsText);
  const liveRemoteConfigFuture =
    /(live remote config|remote config)[\s\S]{0,180}(future work|out of scope|disabled|not live|no live)/i.test(
      docsText
    );
  const automaticPromotionFuture =
    /(automatic release promotion|automatic promotion|auto.?promot)[\s\S]{0,180}(future work|out of scope|not|no)/i.test(
      docsText
    );
  const status =
    phase6FixtureOnly &&
    deployedRolloutFuture &&
    syntheticNotLiveCalendarProof &&
    defaultAvailabilityGated &&
    liveCloudFuture &&
    liveRemoteConfigFuture &&
    automaticPromotionFuture
      ? "PASS"
      : "FAIL";
  return {
    status,
    phase6_fixture_only_documented: phase6FixtureOnly,
    deployed_rollout_future_work_documented: deployedRolloutFuture,
    synthetic_not_live_calendar_proof_documented: syntheticNotLiveCalendarProof,
    default_availability_gated_documented: defaultAvailabilityGated,
    live_cloud_future_work_documented: liveCloudFuture,
    live_remote_config_future_work_documented: liveRemoteConfigFuture,
    automatic_promotion_future_work_documented: automaticPromotionFuture
  };
}

export function parseFixture(repoRoot, fixturePath) {
  if (!fixturePath) {
    return { status: "NOT_PROVIDED", contradictions: [] };
  }
  const absolutePath = path.resolve(repoRoot, fixturePath);
  const fixture = JSON.parse(fs.readFileSync(absolutePath, "utf8"));
  const contradictions = Array.isArray(fixture.contradictions) ? fixture.contradictions : [];
  const phase6Contradictions = contradictions.filter((entry) => {
    return (
      entry.phase === 6 &&
      (entry.claimed_deployed_rollout === true ||
        entry.deployed_rollout === true ||
        /deployed rollout/i.test(`${entry.name ?? ""} ${entry.message ?? ""}`))
    );
  });
  return {
    status: phase6Contradictions.length > 0 ? "FAIL" : "PASS",
    fixture_path: fixturePath,
    contradictions: phase6Contradictions.map((entry) => ({
      name: entry.name ?? "unnamed_contradiction",
      phase: entry.phase,
      message: entry.message ?? "Phase 6 contradiction fixture entry"
    }))
  };
}

export function forbiddenContentScan(report, markdown) {
  const combined = `${JSON.stringify(report)}\n${markdown}`;
  const hits = FORBIDDEN_CONTENT_PATTERNS.filter((entry) => entry.pattern.test(combined)).map((entry) => entry.name);
  return {
    status: hits.length === 0 ? "PASS" : "FAIL",
    checked_categories: FORBIDDEN_CONTENT_PATTERNS.map((entry) => entry.name),
    hits
  };
}
