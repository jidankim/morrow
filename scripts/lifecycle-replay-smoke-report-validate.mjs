import fs from "node:fs";
import path from "node:path";
import {
  assertNoForbiddenFields,
  die,
  operations,
  readJson,
  requireNonEmpty,
} from "./lifecycle-replay-smoke-report-lib.mjs";

function validateTrace(filePath) {
  const lines = fs.readFileSync(filePath, "utf8").split(/\n/).filter(Boolean);
  const seen = new Set(lines.map((line) => JSON.parse(line).span?.operation));
  for (const [operation] of operations) {
    if (!seen.has(operation)) die(`trace missing operation: ${operation}`);
  }
}

function validateStorageReadback(filePath) {
  const value = readJson(filePath);
  const rows = value.readbacks;
  if (value.schema !== "phase3_lifecycle_storage_readback_v1") {
    die("storage readback has unexpected schema");
  }
  if (value.current_run !== true) die("storage readback must be marked current_run");
  if (value.source?.kind !== "sqlite3_readonly_json") {
    die("storage readback must name independent sqlite3 JSON source");
  }
  if (value.source?.source_artifact !== "storage-readback-source.json") {
    die("storage readback must point to storage-readback-source.json");
  }
  if (value.source?.command_log !== "command-logs/storage-readback-sqlite-query.txt") {
    die("storage readback must point to independent command log");
  }
  if (!Array.isArray(rows) || rows.length === 0 || value.row_count !== rows.length) {
    die("storage readback rows are missing or row_count does not match");
  }

  const candidate = rows.find((row) => row.row_type === "candidate");
  if (!candidate || candidate.kind !== "calendar_event" || candidate.state !== "visible") {
    die("storage readback missing visible calendar candidate row");
  }
  if (typeof candidate.current_reason !== "string" || candidate.current_reason.length === 0) {
    die("storage readback candidate row did not prove stored reason readback");
  }
  if (Number(candidate.audit_count) <= 0) {
    die("storage readback candidate row did not prove stored state and audit readback");
  }

  const mapping = rows.find((row) => row.row_type === "external_mapping");
  if (!mapping || mapping.source !== "calendar" || !mapping.external_object_id) {
    die("storage readback missing calendar external mapping row");
  }

  const cursor = rows.find((row) => row.row_type === "replay_cursor");
  if (!cursor || cursor.cursor_stream !== "calendar_proposals" || Number(cursor.cursor_value) < 1) {
    die("storage readback missing advanced replay cursor row");
  }
}

function cargoPassedCount(logText) {
  const matches = logText.matchAll(/test result:\s+ok\.\s+(\d+)\s+passed;/g);
  let sawCargoTestResult = false;
  let passed = 0;
  for (const match of matches) {
    sawCargoTestResult = true;
    passed += Number.parseInt(match[1], 10);
  }
  return { passed, sawCargoTestResult };
}

function validateCommandLogs(outDir, lifecycleReport) {
  const commandLogs = lifecycleReport.command_logs;
  if (!Array.isArray(commandLogs) || commandLogs.length === 0) {
    die("lifecycle-report command_logs must list coverage evidence");
  }

  const receipt = ["scenario: cargo command log pass-count validation"];
  for (const logName of commandLogs) {
    if (typeof logName !== "string" || path.basename(logName) !== logName) {
      die(`invalid command log name in lifecycle-report: ${logName}`);
    }
    const logPath = path.join(outDir, "command-logs", logName);
    requireNonEmpty(logPath);

    if (!logName.startsWith("cargo-")) continue;

    const { passed, sawCargoTestResult } = cargoPassedCount(fs.readFileSync(logPath, "utf8"));
    if (!sawCargoTestResult) {
      die(`cargo evidence log did not contain cargo test result lines: ${logName}`);
    }
    if (passed <= 0) {
      die(`cargo evidence log ran zero tests while listed as coverage evidence: ${logName}`);
    }
    receipt.push(`${logName}: passed=${passed}`);
  }
  receipt.push("result: PASS");
  fs.writeFileSync(path.join(outDir, "command-log-pass-counts.txt"), `${receipt.join("\n")}\n`);
}

export function validate(args) {
  const required = [
    "trace.jsonl",
    "storage-readback.json",
    "storage-readback-source.json",
    "storage-readback.sqlite",
    "lifecycle-report.json",
    "privacy-inspect.txt",
    "canary-rejection.txt",
    "cleanup-receipt.txt",
    "summary.txt",
  ];
  for (const fileName of required) requireNonEmpty(path.join(args.outDir, fileName));
  validateTrace(path.join(args.outDir, "trace.jsonl"));
  assertNoForbiddenFields(path.join(args.outDir, "storage-readback-source.json"));
  const storageReadbackPath = path.join(args.outDir, "storage-readback.json");
  assertNoForbiddenFields(storageReadbackPath);
  validateStorageReadback(storageReadbackPath);
  const lifecycleReportPath = path.join(args.outDir, "lifecycle-report.json");
  const lifecycleReport = readJson(lifecycleReportPath);
  assertNoForbiddenFields(lifecycleReportPath);
  validateCommandLogs(args.outDir, lifecycleReport);
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
