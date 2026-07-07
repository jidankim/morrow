import fs from "node:fs";
import path from "node:path";
import {
  assertCleanArtifact,
  cargoPassedCount,
  die,
  requireNonEmpty,
  vitestPassedCount,
} from "./messages-calendar-approval-trajectory-eval-report-lib.mjs";

const expectedCommandLogs = {
  "cargo-reconcile-trajectory-runner.txt":
    "scripts/run-messages-calendar-approval-trajectory-local-runner.sh --out-dir",
  "cargo-diagnostics-trajectory-trace.txt":
    "cargo test -p morrow-diagnostics --test messages_calendar_approval_trajectory_trace",
  "cargo-storage-decision-evidence.txt":
    "cargo test -p morrow-storage --test decision_evidence",
  "cargo-native-trajectory-eval.txt":
    "cargo test -p morrow --test native_scan_codex trajectory_eval -- --nocapture",
  "npm-status-view.txt": "npm test -- --run StatusView",
  "npm-messages-tauri-commands.txt": "npm test -- --run messagesTauriCommands",
};

function validateCommandLogName(logName, seenLogs) {
  if (typeof logName !== "string" || path.basename(logName) !== logName) {
    die(`invalid command log name in report: ${logName}`);
  }
  if (seenLogs.has(logName)) die(`duplicate command log in report: ${logName}`);
  const expectedInvocation = expectedCommandLogs[logName];
  if (!expectedInvocation) die(`unexpected command log in report: ${logName}`);
  seenLogs.add(logName);
  return expectedInvocation;
}

function validateCommandLogText(logName, logText, expectedInvocation) {
  if (!logText.includes("scenario:") || !logText.includes(`invocation: ${expectedInvocation}`)) {
    die(`command log missing expected scenario/invocation marker: ${logName}`);
  }
  if (logName.startsWith("cargo-")) {
    const { passed, sawCargoTestResult } = cargoPassedCount(logText);
    if (!sawCargoTestResult || passed <= 0) {
      die(`cargo evidence log did not prove nonzero passed tests: ${logName}`);
    }
    return passed;
  }
  if (logName.startsWith("npm-")) {
    const passed = vitestPassedCount(logText);
    if (passed <= 0) die(`vitest evidence log did not prove nonzero passed tests: ${logName}`);
    return passed;
  }
  die(`unsupported command log type: ${logName}`);
}

export function validateCommandLogs(outDir, report) {
  const commandLogs = report.command_logs;
  if (!Array.isArray(commandLogs) || commandLogs.length === 0) {
    die("report command_logs must list smoke evidence");
  }
  if (commandLogs.length !== Object.keys(expectedCommandLogs).length) {
    die("report command_logs must list every expected smoke command");
  }
  const seenLogs = new Set();
  const receipt = ["scenario: phase 5 command log pass-count validation"];
  for (const logName of commandLogs) {
    const expectedInvocation = validateCommandLogName(logName, seenLogs);
    const logPath = path.join(outDir, "command-logs", logName);
    requireNonEmpty(logPath);
    assertCleanArtifact(logPath);
    const passed = validateCommandLogText(
      logName,
      fs.readFileSync(logPath, "utf8"),
      expectedInvocation,
    );
    receipt.push(`${logName}: invocation_marker_present=yes passed=${passed}`);
  }
  for (const expectedLog of Object.keys(expectedCommandLogs)) {
    if (!seenLogs.has(expectedLog)) die(`missing expected command log: ${expectedLog}`);
  }
  receipt.push("result: PASS");
  fs.writeFileSync(path.join(outDir, "command-log-pass-counts.txt"), `${receipt.join("\n")}\n`);
}
