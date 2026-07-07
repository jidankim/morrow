import { execFile } from "node:child_process";
import { readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { promisify } from "node:util";

import { exists, readIfPresent } from "./beta-gate-io.mjs";

const run = promisify(execFile);
const defaultTimeoutMs = 120_000;

export function createGateState({ repoRoot, startedAt }) {
  return {
    repoRoot,
    startedAt,
    checks: [],
    missingPrerequisites: new Set(),
  };
}

export function addMissing(state, ...items) {
  for (const item of items) {
    if (item) {
      state.missingPrerequisites.add(item);
    }
  }
}

export async function gitValue(repoRoot, args) {
  try {
    const { stdout } = await run("git", args, { cwd: repoRoot, timeout: 10_000 });
    return stdout.trim();
  } catch {
    return "";
  }
}

export async function dirtyPaths(repoRoot) {
  try {
    const { stdout } = await run("git", ["status", "--short"], { cwd: repoRoot, timeout: 10_000 });
    return stdout.split("\n").map((line) => line.trim()).filter(Boolean);
  } catch {
    return [];
  }
}

function statusFromFailure(stdout, stderr, evidenceText) {
  const text = `${stdout}\n${stderr}\n${evidenceText}`;
  if (
    /RESULT:\s*BLOCKED|BLOCKED|missing|does not exist|not configured|diagnostic artifacts are not beta-ready|could not be read|not found|found 0/i.test(
      text,
    )
  ) {
    return "BLOCKED";
  }
  return "FAIL";
}

function collectMissingFromText(text) {
  const found = [];
  for (const name of ["APPLE_SIGNING_IDENTITY", "APPLE_ID", "APPLE_PASSWORD", "APPLE_TEAM_ID"]) {
    if (new RegExp(`missing[^\\n]*${name}|${name}[^\\n]*missing`, "i").test(text)) {
      found.push(name);
    }
  }
  if (/bundle directory.*does not exist|bundle directory exists.*FAIL|bundle directory exists.*BLOCKED/i.test(text)) {
    found.push("beta bundle directory");
  }
  if (/exactly one Morrow\.app.*found 0|no Morrow\.app/i.test(text)) {
    found.push("signed Morrow.app beta artifact");
  }
  if (/exactly one DMG.*found 0|no DMG/i.test(text)) {
    found.push("signed/notarized DMG beta artifact");
  }
  if (/notarization|stapler|stapling/i.test(text) && !/PASS.*stapler|RESULT:\s*PASS/i.test(text)) {
    found.push("notarized/stapled beta artifact");
  }
  if (/ad-hoc|adhoc|diagnostic artifacts are not beta-ready/i.test(text)) {
    found.push("non-diagnostic release-signed artifact");
  }
  if (/manifest/i.test(text) && /missing|not found|could not be read|FAIL/i.test(text)) {
    found.push("beta release manifest/checklist");
  }
  return found;
}

export async function runCheck(state, { name, command, args, evidencePath, timeoutMs = defaultTimeoutMs, retainOnBlocked = false }) {
  const stdoutPath = evidencePath.replace(/\.[^.]+$/, ".stdout");
  let stdout = "";
  let stderr = "";
  let exitCode = 0;
  let timedOut = false;
  try {
    const result = await run(command, args, {
      cwd: state.repoRoot,
      timeout: timeoutMs,
      maxBuffer: 10 * 1024 * 1024,
    });
    stdout = result.stdout;
    stderr = result.stderr;
  } catch (error) {
    stdout = error.stdout ?? "";
    stderr = error.stderr ?? error.message;
    exitCode = typeof error.code === "number" ? error.code : 1;
    timedOut = error.killed || error.signal === "SIGTERM";
  }
  await writeFile(stdoutPath, `${stdout}${stderr ? `\n${stderr}` : ""}`, "utf8");
  const evidenceText = await readIfPresent(evidencePath);
  const status = exitCode === 0 ? "PASS" : statusFromFailure(stdout, stderr, evidenceText);
  if (status !== "PASS") {
    addMissing(state, ...collectMissingFromText(`${stdout}\n${stderr}\n${evidenceText}`));
  }
  const check = {
    name,
    status,
    command: [command, ...args].join(" "),
    exit_code: exitCode,
    timed_out: timedOut,
    stdout_path: path.relative(state.repoRoot, stdoutPath),
    evidence_path: path.relative(state.repoRoot, evidencePath),
  };
  state.checks.push(check);
  if (status === "PASS" && !retainOnBlocked) {
    check.evidence_retained_only_on_pass = true;
  }
  return check;
}

export async function verifyEvidenceFile(state, { name, filePath, requiredPatterns, forbiddenPatterns }) {
  const check = {
    name,
    command: `verify evidence ${filePath}`,
    evidence_path: path.relative(state.repoRoot, filePath),
    stdout_path: null,
    exit_code: 0,
    timed_out: false,
  };
  const info = await exists(filePath);
  if (!info?.isFile()) {
    check.status = "BLOCKED";
    check.details = `${filePath} is missing`;
    check.exit_code = 1;
    addMissing(state, filePath);
    state.checks.push(check);
    return;
  }
  const text = await readFile(filePath, "utf8");
  const missing = requiredPatterns.filter((pattern) => !pattern.test(text));
  const forbidden = forbiddenPatterns.filter((pattern) => pattern.test(text));
  if (missing.length > 0 || forbidden.length > 0) {
    check.status = forbidden.length > 0 ? "FAIL" : "BLOCKED";
    check.details = `missing ${missing.map(String).join(", ") || "none"}; forbidden ${forbidden.map(String).join(", ") || "none"}`;
    check.exit_code = 1;
    if (missing.length > 0) {
      addMissing(state, `${name} PASS evidence`);
    }
  } else {
    check.status = "PASS";
    check.details = "evidence file is present and redaction checks passed";
  }
  state.checks.push(check);
}

export async function suppressPassEvidenceOnBlockedRun(state) {
  for (const check of state.checks) {
    if (check.status === "PASS" && check.evidence_retained_only_on_pass && check.evidence_path) {
      await rm(path.join(state.repoRoot, check.evidence_path), { force: true });
      check.evidence_suppressed_on_blocked_run = true;
      check.details =
        "PASS sub-evidence was suppressed because this beta gate run is blocked; stdout transcript remains the captured artifact";
    }
  }
}
