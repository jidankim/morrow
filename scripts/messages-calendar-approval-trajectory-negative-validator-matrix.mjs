#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const root = process.cwd();
const sourceDir =
  ".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke";
const caseRoot =
  ".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/scoring/negative-validator-cases";
const matrixPath =
  ".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/scoring/negative-validator-matrix.txt";

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function writeJson(filePath, value) {
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

function mutateJson(caseDir, fileName, update) {
  const filePath = path.join(caseDir, fileName);
  const value = readJson(filePath);
  update(value);
  writeJson(filePath, value);
}

function copyCase(caseName) {
  const caseDir = path.join(caseRoot, caseName);
  fs.cpSync(sourceDir, caseDir, { recursive: true });
  return caseDir;
}

function removeFamilyRows(caseDir, family) {
  mutateJson(caseDir, "trajectory-report.json", (report) => {
    delete report.coverage.required_case_families[family];
    report.scores.cases = report.scores.cases.filter((item) => item.family !== family);
  });
}

function duplicateScoreFamily(caseDir) {
  mutateJson(caseDir, "trajectory-report.json", (report) => {
    report.scores.cases[1] = { ...report.scores.cases[0] };
  });
}

function runValidator(caseName, caseDir) {
  const result = spawnSync(
    "node",
    [
      "scripts/messages-calendar-approval-trajectory-eval-report.mjs",
      "validate",
      "--out-dir",
      caseDir,
    ],
    { cwd: root, encoding: "utf8" },
  );
  const output = [
    `scenario: ${caseName}`,
    `invocation: node scripts/messages-calendar-approval-trajectory-eval-report.mjs validate --out-dir ${caseDir}`,
    `exit_status: ${result.status}`,
    "stdout:",
    result.stdout.trimEnd(),
    "stderr:",
    result.stderr.trimEnd(),
  ].join("\n");
  fs.writeFileSync(path.join(caseDir, "validate.txt"), `${output}\n`);
  if (result.status === 0) return `${caseName}: FAIL accepted with status 0`;
  return `${caseName}: PASS rejected with status ${result.status}: ${result.stderr.trim()}`;
}

const cases = [
  ["missing-required-family", (caseDir) => removeFamilyRows(caseDir, "task_reminder_accepted")],
  [
    "missing-collateral-check",
    (caseDir) =>
      mutateJson(caseDir, "trajectory-report.json", (report) => {
        delete report.coverage.collateral_damage_checks;
      }),
  ],
  [
    "missing-replay-score",
    (caseDir) =>
      mutateJson(caseDir, "trajectory-report.json", (report) => {
        report.coverage.replay_scoring.idempotent_retry_score = 0;
      }),
  ],
  [
    "stale-non-current",
    (caseDir) =>
      mutateJson(caseDir, "trajectory-report.json", (report) => {
        report.current_run = false;
      }),
  ],
  [
    "forbidden-raw-content-trace",
    (caseDir) =>
      fs.appendFileSync(
        path.join(caseDir, "trace.jsonl"),
        [
          '{"schema_version":"v1","trajectory_case_id":"phase5:scheduled_meeting_accepted:v1",',
          '"current_run":true,"span":{"replay_run_id":"phase5-trajectory-current-run"},',
          '"raw_text":"[REDACTED]"}\n',
        ].join(""),
      ),
  ],
  [
    "forbidden-raw-content-summary",
    (caseDir) => fs.appendFileSync(path.join(caseDir, "summary.txt"), "raw_text: [REDACTED]\n"),
  ],
  [
    "zero-misleading-command-log",
    (caseDir) =>
      fs.writeFileSync(
        path.join(caseDir, "command-logs/cargo-reconcile-trajectory-runner.txt"),
        "test result: ok. 2 passed;\n",
      ),
  ],
  [
    "missing-canary-rejection",
    (caseDir) => fs.writeFileSync(path.join(caseDir, "canary-rejection.txt"), ""),
  ],
  [
    "missing-cleanup-receipt",
    (caseDir) => fs.rmSync(path.join(caseDir, "cleanup-receipt.txt"), { force: true }),
  ],
  ["duplicate-score-family-case-data", duplicateScoreFamily],
  [
    "missing-no-live-backend-flag",
    (caseDir) =>
      mutateJson(caseDir, "trajectory-report.json", (report) => {
        report.no_live_backend_use = false;
      }),
  ],
];

const matrix = [];
fs.rmSync(caseRoot, { recursive: true, force: true });
fs.mkdirSync(caseRoot, { recursive: true });
for (const [caseName, mutate] of cases) {
  const caseDir = copyCase(caseName);
  mutate(caseDir);
  matrix.push(runValidator(caseName, caseDir));
}

fs.writeFileSync(matrixPath, `${matrix.join("\n")}\n`);
console.log(`wrote ${matrixPath}`);
