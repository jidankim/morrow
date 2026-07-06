import fs from "node:fs";
import path from "node:path";
import { validateCommandLogs } from "./messages-calendar-approval-trajectory-eval-report-command-logs.mjs";
import {
  assertCleanArtifact,
  familyFromCaseId,
  die,
  readJson,
  reportName,
  requireNonEmpty,
  requiredCaseFamilies,
} from "./messages-calendar-approval-trajectory-eval-report-lib.mjs";

const requiredArtifactNames = [
  "trace.jsonl",
  "storage-readback.json",
  "decision-evidence.json",
  reportName,
  "privacy-inspect.txt",
  "canary-rejection.txt",
  "cleanup-receipt.txt",
  "summary.txt",
  "local-runner-source/local-trajectory-run.json",
  "local-runner-source/live-surface-rejection.json",
];

function validateFreshCurrentRun(value, label) {
  if (value.current_run !== true) die(`${label} must be marked current_run`);
  const generatedAt = Date.parse(value.generated_at_utc);
  if (!Number.isFinite(generatedAt)) die(`${label} missing parseable generated_at_utc`);
  const ageMs = Date.now() - generatedAt;
  if (ageMs < 0 || ageMs > 7 * 24 * 60 * 60 * 1000) {
    die(`${label} generated_at_utc is stale or from the future`);
  }
}

function validateTrace(filePath) {
  const lines = fs.readFileSync(filePath, "utf8").split(/\n/).filter(Boolean);
  if (lines.length < Object.keys(requiredCaseFamilies).length) {
    die("trace must cover every trajectory case family");
  }
  const cases = new Set();
  for (const line of lines) {
    const record = JSON.parse(line);
    if (record.current_run !== true) die("trace record must be current_run");
    if (record.span?.replay_run_id !== "phase5-trajectory-current-run") {
      die("trace record missing current replay_run_id marker");
    }
    const family = familyFromCaseId(record.trajectory_case_id);
    if (!requiredCaseFamilies[family]) die(`trace contains unknown case: ${record.trajectory_case_id}`);
    cases.add(record.trajectory_case_id);
  }
  for (const family of Object.values(requiredCaseFamilies)) {
    if (!cases.has(family.caseId)) die(`trace missing case coverage: ${family.caseId}`);
  }
}

function validateUniqueRequiredCases(rows, label) {
  const families = new Set();
  const caseIds = new Set();
  for (const row of rows) {
    const family = row.family ?? familyFromCaseId(row.trajectory_case_id);
    const expected = requiredCaseFamilies[family];
    if (!expected) die(`${label} contains unknown family: ${family}`);
    if (row.trajectory_case_id !== expected.caseId) {
      die(`${label} case id mismatch for family: ${family}`);
    }
    if (families.has(family)) die(`${label} duplicates family: ${family}`);
    if (caseIds.has(row.trajectory_case_id)) {
      die(`${label} duplicates case id: ${row.trajectory_case_id}`);
    }
    families.add(family);
    caseIds.add(row.trajectory_case_id);
  }
  for (const [family, expected] of Object.entries(requiredCaseFamilies)) {
    if (!families.has(family)) die(`${label} missing required family: ${family}`);
    if (!caseIds.has(expected.caseId)) die(`${label} missing required case id: ${expected.caseId}`);
  }
}

function validateStorageReadback(filePath) {
  const value = readJson(filePath);
  if (value.schema !== "phase5_messages_calendar_approval_trajectory_storage_readback_v1") {
    die("storage readback has unexpected schema");
  }
  validateFreshCurrentRun(value, "storage readback");
  if (value.source?.source_artifact !== "local-runner-source/local-trajectory-run.json") {
    die("storage readback must point to local runner source artifact");
  }
  const rows = Array.isArray(value.readbacks) ? value.readbacks : [];
  if (rows.length !== Object.keys(requiredCaseFamilies).length || value.row_count !== rows.length) {
    die("storage readback row_count must match required trajectory cases");
  }
  validateUniqueRequiredCases(rows, "storage readback");
  if (!rows.some((row) => row.replay_score === 100)) die("storage readback missing replay score");
  if (!rows.some((row) => row.collateral_damage_score === 100)) {
    die("storage readback missing collateral-damage score");
  }
}

function validateDecisionEvidence(filePath) {
  const value = readJson(filePath);
  if (value.schema !== "phase5_messages_calendar_approval_trajectory_decision_evidence_v1") {
    die("decision evidence has unexpected schema");
  }
  validateFreshCurrentRun(value, "decision evidence");
  const items = Array.isArray(value.items) ? value.items : [];
  if (items.length !== Object.keys(requiredCaseFamilies).length) {
    die("decision evidence must cover every trajectory case");
  }
  const families = new Set();
  for (const item of items) {
    if (item.route !== "phase5_local_trajectory_eval") {
      die("decision evidence item has unexpected route");
    }
    if (item.traceRetention !== "retained") die("decision evidence must prove retained trace");
    if (!Array.isArray(item.traceSequence) || item.traceSequence.length === 0) {
      die("decision evidence item missing trace sequence");
    }
    if (!requiredCaseFamilies[item.labelValue]) {
      die(`decision evidence contains unknown family: ${item.labelValue}`);
    }
    if (families.has(item.labelValue)) die(`decision evidence duplicates family: ${item.labelValue}`);
    families.add(item.labelValue);
  }
  for (const family of Object.keys(requiredCaseFamilies)) {
    if (!families.has(family)) die(`decision evidence missing required family: ${family}`);
  }
}

function validateRequiredCoverage(report) {
  const coverage = report.coverage?.required_case_families;
  for (const [family, expected] of Object.entries(requiredCaseFamilies)) {
    const actual = coverage?.[family];
    if (actual?.covered !== true) die(`report missing required case family: ${family}`);
    if (actual.trajectory_case_id !== expected.caseId) {
      die(`report case id mismatch for family: ${family}`);
    }
    if (actual.required_outcome !== expected.requiredOutcome) {
      die(`report required outcome mismatch for family: ${family}`);
    }
  }
  if (report.coverage?.collateral_damage_checks?.non_target_preserved !== true) {
    die("report missing collateral-damage checks");
  }
  if (report.coverage?.replay_scoring?.idempotent_retry_score !== 100) {
    die("report missing replay score");
  }
  if (report.coverage?.cleanup?.synthetic_privacy_surfaces_removed !== true) {
    die("report missing cleanup coverage");
  }
}

function validateScores(report) {
  const thresholds = report.thresholds;
  if (report.scores?.overall < thresholds?.overall_min_score) die("overall score below threshold");
  const cases = Array.isArray(report.scores?.cases) ? report.scores.cases : [];
  if (cases.length !== Object.keys(requiredCaseFamilies).length) {
    die("report score cases must match required case count");
  }
  validateUniqueRequiredCases(cases, "report score cases");
  for (const caseScore of cases) {
    if (caseScore.score < thresholds.per_case_min_score) {
      die(`case score below threshold: ${caseScore.trajectory_case_id}`);
    }
  }
  const replay = cases.find((caseScore) => caseScore.family === "replay_idempotent_retry");
  if (!replay || replay.replay_score < thresholds.replay_min_score) {
    die("replay score below threshold");
  }
  const collateral = cases.find(
    (caseScore) => caseScore.family === "collateral_damage_non_target_preserved",
  );
  if (!collateral || collateral.collateral_damage_score < thresholds.collateral_damage_min_score) {
    die("collateral-damage score below threshold");
  }
}

function validateReport(filePath) {
  const report = readJson(filePath);
  if (report.schema !== "phase5_messages_calendar_approval_trajectory_eval_report_v1") {
    die("trajectory report has unexpected schema");
  }
  validateFreshCurrentRun(report, "trajectory report");
  if (report.run_marker !== "phase5-trajectory-current-run") die("report missing run marker");
  if (report.no_live_backend_use !== true || report.no_live_surfaces_used !== true) {
    die("report does not prove no live backend/surface use");
  }
  validateRequiredCoverage(report);
  validateScores(report);
  return report;
}

export function validate(args) {
  for (const fileName of requiredArtifactNames) requireNonEmpty(path.join(args.outDir, fileName));
  validateTrace(path.join(args.outDir, "trace.jsonl"));
  for (const fileName of requiredArtifactNames) assertCleanArtifact(path.join(args.outDir, fileName));
  validateStorageReadback(path.join(args.outDir, "storage-readback.json"));
  validateDecisionEvidence(path.join(args.outDir, "decision-evidence.json"));
  const report = validateReport(path.join(args.outDir, reportName));
  validateCommandLogs(args.outDir, report);
  requireNonEmpty(path.join(args.outDir, "command-log-pass-counts.txt"));
  const privacy = fs.readFileSync(path.join(args.outDir, "privacy-inspect.txt"), "utf8");
  if (!privacy.includes("privacy inspection passed")) die("privacy-inspect did not report pass");
  const canary = fs.readFileSync(path.join(args.outDir, "canary-rejection.txt"), "utf8");
  if (!canary.includes("result: PASS") || !canary.includes("exited non-zero before sanitization")) {
    die("canary rejection artifact did not prove expected rejection");
  }
  const cleanup = fs.readFileSync(path.join(args.outDir, "cleanup-receipt.txt"), "utf8");
  if (!cleanup.includes("result: PASS") || !cleanup.includes("removed: synthetic privacy surface")) {
    die("cleanup receipt did not prove synthetic privacy surfaces were removed");
  }
}
