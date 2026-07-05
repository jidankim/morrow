import fs from "node:fs";
import path from "node:path";
import {
  assertNoForbiddenFields,
  cargoPassedCount,
  correctionTrace,
  die,
  readJson,
  requireNonEmpty,
  requiredOutcomeCoverage,
  vitestPassedCount,
} from "./human-approval-correction-smoke-report-lib.mjs";

function validateTrace(filePath) {
  const lines = fs.readFileSync(filePath, "utf8").split(/\n/).filter(Boolean);
  if (lines.length !== 1) die("trace must contain exactly one current-run correction record");
  const record = JSON.parse(lines[0]);
  if (record.span?.component !== correctionTrace.component) die("trace missing correction component");
  if (record.span?.operation !== correctionTrace.operation) die("trace missing user_correction operation");
  if (record.span?.decision !== correctionTrace.decision) die("trace missing user_corrected decision");
  if (record.span?.outcome !== correctionTrace.outcome) die("trace missing noop outcome");
}

function validateDecisionEvidence(filePath) {
  const value = readJson(filePath);
  if (!Array.isArray(value.items) || value.items.length !== 1) {
    die("decision evidence must contain one correction item");
  }
  const item = value.items[0];
  if (item.route !== "human_correction") die("decision evidence missing human_correction route");
  if (item.reasonCode !== correctionTrace.reasonCode) die("decision evidence missing correction reason");
  if (item.labelType !== correctionTrace.labelType) die("decision evidence missing field_quality label");
  if (item.labelValue !== correctionTrace.labelValue) die("decision evidence missing title_edited label");
  if (item.traceRetention !== "retained") die("decision evidence must prove retained trace");
  const traceSequence = Array.isArray(item.traceSequence) ? item.traceSequence : [];
  const correction = traceSequence.find((step) => step.operation === correctionTrace.operation);
  if (!correction || correction.decision !== correctionTrace.decision) {
    die("decision evidence trace sequence missing user correction outcome");
  }
}

function validateStorageReadback(filePath) {
  const value = readJson(filePath);
  if (value.schema !== "phase4_human_approval_correction_storage_readback_v1") {
    die("storage readback has unexpected schema");
  }
  if (value.current_run !== true) die("storage readback must be marked current_run");
  if (value.source?.source_artifact !== "decision-evidence.json") {
    die("storage readback must point to decision-evidence.json");
  }
  if (value.source?.command_log !== "command-logs/cargo-native-decision-evidence.txt") {
    die("storage readback must point to native decision evidence command log");
  }
  const rows = value.readbacks;
  if (!Array.isArray(rows) || rows.length !== 1 || value.row_count !== rows.length) {
    die("storage readback rows are missing or row_count does not match");
  }
  const row = rows[0];
  if (row.route !== "human_correction" || row.reason_code !== correctionTrace.reasonCode) {
    die("storage readback missing human correction route/readback");
  }
  if (row.label_type !== correctionTrace.labelType || row.label_value !== correctionTrace.labelValue) {
    die("storage readback missing correction label readback");
  }
  if (row.trace_operation !== correctionTrace.operation || row.trace_decision !== correctionTrace.decision) {
    die("storage readback missing correction trace readback");
  }
}

function validateReport(filePath) {
  const report = readJson(filePath);
  if (report.schema !== "phase4_human_approval_correction_smoke_report_v1") {
    die("human approval correction report has unexpected schema");
  }
  if (report.current_run !== true) die("human approval correction report must be current_run");
  if (!report.generated_at_utc) die("human approval correction report missing generated_at_utc");
  if (report.coverage?.user_correction !== true) die("report missing user_correction coverage");
  const outcomes = report.coverage?.outcomes;
  if (
    outcomes?.user_corrected !== true ||
    outcomes?.noop !== true ||
    outcomes?.field_quality !== true ||
    outcomes?.title_edited !== true ||
    outcomes?.trace_retained !== true
  ) {
    die("report missing correction outcomes coverage");
  }
  for (const outcome of Object.keys(requiredOutcomeCoverage)) {
    if (outcomes?.[outcome] !== true) {
      die(`report missing required outcome coverage: ${outcome}`);
    }
  }
  const requiredOutcomes = report.coverage?.required_outcomes;
  for (const [outcome, expected] of Object.entries(requiredOutcomeCoverage)) {
    const actual = requiredOutcomes?.[outcome];
    if (actual?.covered !== true) {
      die(`report missing required outcome coverage: ${outcome}`);
    }
    if (actual.label_type !== expected.labelType || actual.label_value !== expected.labelValue) {
      die(`report required outcome coverage mismatch: ${outcome}`);
    }
    if (actual.evidence_artifact !== expected.artifact) {
      die(`report required outcome evidence mismatch: ${outcome}`);
    }
    if (!Array.isArray(report.command_logs) || !report.command_logs.includes(path.basename(expected.artifact))) {
      die(`report command_logs missing required outcome evidence: ${outcome}`);
    }
    if (actual.observable !== expected.observable) {
      die(`report required outcome observable mismatch: ${outcome}`);
    }
  }
  if (report.coverage?.cleanup?.synthetic_privacy_surfaces_removed !== true) {
    die("report missing cleanup coverage");
  }
  return report;
}

function validateCommandLogs(outDir, report) {
  const commandLogs = report.command_logs;
  if (!Array.isArray(commandLogs) || commandLogs.length === 0) {
    die("report command_logs must list smoke evidence");
  }

  const receipt = ["scenario: phase 4 command log pass-count validation"];
  for (const logName of commandLogs) {
    if (typeof logName !== "string" || path.basename(logName) !== logName) {
      die(`invalid command log name in report: ${logName}`);
    }
    const logPath = path.join(outDir, "command-logs", logName);
    requireNonEmpty(logPath);
    const logText = fs.readFileSync(logPath, "utf8");
    for (const [outcome, expected] of Object.entries(requiredOutcomeCoverage)) {
      if (logName === path.basename(expected.artifact) && !logText.includes(expected.observable)) {
        die(`required outcome evidence log missing observable for ${outcome}: ${logName}`);
      }
    }
    if (logName.startsWith("cargo-")) {
      const { passed, sawCargoTestResult } = cargoPassedCount(logText);
      if (!sawCargoTestResult || passed <= 0) {
        die(`cargo evidence log did not prove nonzero passed tests: ${logName}`);
      }
      receipt.push(`${logName}: passed=${passed}`);
    }
    if (logName.startsWith("npm-")) {
      const passed = vitestPassedCount(logText);
      if (passed <= 0) die(`vitest evidence log did not prove nonzero passed tests: ${logName}`);
      receipt.push(`${logName}: passed=${passed}`);
    }
  }
  receipt.push("result: PASS");
  fs.writeFileSync(path.join(outDir, "command-log-pass-counts.txt"), `${receipt.join("\n")}\n`);
}

export function validate(args) {
  const required = [
    "trace.jsonl",
    "storage-readback.json",
    "decision-evidence.json",
    "human-approval-correction-report.json",
    "privacy-inspect.txt",
    "canary-rejection.txt",
    "cleanup-receipt.txt",
    "summary.txt",
  ];
  for (const fileName of required) requireNonEmpty(path.join(args.outDir, fileName));
  validateTrace(path.join(args.outDir, "trace.jsonl"));
  for (const fileName of ["storage-readback.json", "decision-evidence.json", "human-approval-correction-report.json"]) {
    assertNoForbiddenFields(path.join(args.outDir, fileName));
  }
  validateStorageReadback(path.join(args.outDir, "storage-readback.json"));
  validateDecisionEvidence(path.join(args.outDir, "decision-evidence.json"));
  const report = validateReport(path.join(args.outDir, "human-approval-correction-report.json"));
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
