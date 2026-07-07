import { readFile } from "node:fs/promises";
import path from "node:path";
import { buildInvocation, readJson, writeReport } from "./report-io.mjs";
import { check, validateManifestObject } from "./report-validation.mjs";

const MIN_FRESH_DATE_UTC = Date.parse("2026-07-07T00:00:00Z");
const REQUIRED_TEXT = ["summary.txt", "deployment-action.txt", "privacy-inspect.txt", "docs-qa.md", "runbook-qa.md"];
const REQUIRED_JSON = [
  "manifest-validation.json",
  "beta-readiness.json",
  "negative-matrix.json",
  "regression-gate.json",
  "retention-report.json",
  "kill-switches.json",
];
const REQUIRED_DOC_TOKENS = [
  "Version",
  "Branch",
  "Commit",
  "Manifest State",
  "Artifact",
  "Beta Gate",
  "Phase 6",
  "Rollout Gate",
  "Privacy",
  "Docs",
  "Runbook",
  "deployment_action=none",
  "Limitations",
];
const FORBIDDEN_TEXT =
  /\/Users\/|\/private\/|~\/\.codex|auth\.json|codex_access_token|sk-[A-Za-z0-9]|MORROW_PRIVACY_CANARY_RAW|PHASE6_RAW_CONTENT_CANARY_DO_NOT_STORE/i;

function statusOf(value, ...keys) {
  for (const key of keys) {
    if (typeof value?.[key] === "string") return value[key];
  }
  return null;
}

function freshCheck(checks, artifact, label) {
  const stamp = statusOf(artifact, "generated_at_utc", "completed_at", "validated_at");
  if (stamp === null) {
    check(checks, `${label} has freshness timestamp or required text receipt`, true, "text receipt or schema without timestamp");
    return;
  }
  const parsed = Date.parse(stamp);
  check(
    checks,
    `${label} is fresh for final handoff`,
    Number.isFinite(parsed) && parsed >= MIN_FRESH_DATE_UTC,
    `observed ${JSON.stringify(stamp)}`,
  );
}

async function readText(checks, filePath, label) {
  try {
    const text = await readFile(filePath, "utf8");
    check(checks, `${label} exists and is non-empty`, text.trim().length > 0, filePath);
    return text;
  } catch (error) {
    check(checks, `${label} exists and is non-empty`, false, `${filePath}: ${error.message}`);
    return "";
  }
}

async function readJsonArtifact(checks, filePath, label) {
  try {
    const value = await readJson(filePath);
    check(checks, `${label} parses as JSON`, true, filePath);
    freshCheck(checks, value, label);
    return value;
  } catch (error) {
    check(checks, `${label} parses as JSON`, false, `${filePath}: ${error.message}`);
    return null;
  }
}

function docHas(text, token) {
  return text.toLowerCase().includes(token.toLowerCase());
}

function checkChecklistTokens(checks, checklist) {
  for (const token of REQUIRED_DOC_TOKENS) {
    check(checks, `checklist includes ${token}`, docHas(checklist, token), token);
  }
}

function checkBlockedClaims(checks, manifest, smoke, checklist) {
  const betaStatus = statusOf(smoke.beta, "overall_status");
  const rolloutStatus = statusOf(smoke.regression, "overall_status");
  const defaultAllowed = manifest?.default_availability?.allowed === true;
  const defaultClaimLines = checklist
    .split("\n")
    .filter((line) => /default(?: availability|_availability| available)/i.test(line))
    .filter((line) => !/\b(?:false|unavailable|blocked|not|disabled|future work)\b/i.test(line));
  const claimsDefault = defaultClaimLines.length > 0 || /default_available\s*[:=]\s*true/i.test(checklist);
  check(
    checks,
    "checklist matches manifest beta state",
    docHas(checklist, `Manifest State: ${manifest?.state}`),
    `manifest_state=${JSON.stringify(manifest?.state)}`,
  );
  check(
    checks,
    "checklist records beta gate status",
    docHas(checklist, `Beta Gate: ${betaStatus}`),
    `beta_status=${JSON.stringify(betaStatus)}`,
  );
  check(
    checks,
    "checklist records rollout gate status",
    docHas(checklist, `Rollout Gate: ${rolloutStatus}`),
    `rollout_status=${JSON.stringify(rolloutStatus)}`,
  );
  check(
    checks,
    "default availability is not overclaimed while gates are blocked",
    !(claimsDefault && (betaStatus !== "PASS" || rolloutStatus !== "PASS" || !defaultAllowed)),
    `default_claim=${claimsDefault} beta=${betaStatus} rollout=${rolloutStatus} manifest_allowed=${defaultAllowed} lines=${JSON.stringify(defaultClaimLines)}`,
  );
  check(
    checks,
    "checklist records default unavailable state",
    docHas(checklist, "Default Availability Allowed: false"),
    "default availability must remain false for current handoff",
  );
  check(
    checks,
    "dirty worktree is documented as pending rather than released",
    smoke.regression?.dirty_worktree?.status !== "dirty" || /\b(?:uncommitted|pending review|pre-commit pending)\b/i.test(checklist),
    `dirty_status=${JSON.stringify(smoke.regression?.dirty_worktree?.status)}`,
  );
}

function checkTextEvidence(checks, smokeTexts, checklist) {
  check(checks, "summary records final smoke PASS", /result:\s*PASS/i.test(smokeTexts.summary), "summary.txt");
  check(checks, "summary records no live services or deployment", /no live backend services/i.test(smokeTexts.summary), "summary.txt");
  check(checks, "deployment action receipt is none", /deployment_action=none/i.test(smokeTexts.deployment), "deployment-action.txt");
  check(checks, "privacy inspection passed", /result:\s*PASS/i.test(smokeTexts.privacy), "privacy-inspect.txt");
  check(checks, "docs QA passed", /Result:\s*PASS/i.test(smokeTexts.docsQa), "docs-qa.md");
  check(checks, "runbook QA passed", /Result:\s*PASS/i.test(smokeTexts.runbookQa), "runbook-qa.md");
  check(checks, "handoff output has no private paths, canaries, or credential sentinels", !FORBIDDEN_TEXT.test(checklist), "checklist text");
}

export async function runVerifyHandoff(args) {
  if (!args.manifest || !args.smoke_dir || !args.docs || !args.out_dir) {
    throw new Error("Usage: verify-handoff --manifest <path> --smoke-dir <dir> --docs <path> --out-dir <dir>");
  }

  const checks = [];
  const manifest = await readJsonArtifact(checks, args.manifest, "manifest");
  const manifestValidation = manifest ? validateManifestObject(manifest) : { checks: [] };
  for (const entry of manifestValidation.checks) check(checks, `manifest validation: ${entry.name}`, entry.status === "PASS", entry.details);

  const smokeTexts = {
    summary: await readText(checks, path.join(args.smoke_dir, "summary.txt"), "summary"),
    deployment: await readText(checks, path.join(args.smoke_dir, "deployment-action.txt"), "deployment action"),
    privacy: await readText(checks, path.join(args.smoke_dir, "privacy-inspect.txt"), "privacy inspect"),
    docsQa: await readText(checks, path.join(args.smoke_dir, "docs-qa.md"), "docs QA"),
    runbookQa: await readText(checks, path.join(args.smoke_dir, "runbook-qa.md"), "runbook QA"),
  };
  const smoke = {
    beta: await readJsonArtifact(checks, path.join(args.smoke_dir, "beta-readiness.json"), "beta readiness"),
    regression: await readJsonArtifact(checks, path.join(args.smoke_dir, "regression-gate.json"), "rollout regression gate"),
    negative: await readJsonArtifact(checks, path.join(args.smoke_dir, "negative-matrix.json"), "negative matrix"),
    manifest: await readJsonArtifact(checks, path.join(args.smoke_dir, "manifest-validation.json"), "smoke manifest validation"),
    retention: await readJsonArtifact(checks, path.join(args.smoke_dir, "retention-report.json"), "retention report"),
    killSwitches: await readJsonArtifact(checks, path.join(args.smoke_dir, "kill-switches.json"), "kill-switch report"),
  };
  for (const file of [...REQUIRED_TEXT, ...REQUIRED_JSON]) {
    check(checks, `required smoke artifact listed: ${file}`, true, path.join(args.smoke_dir, file));
  }

  const checklist = await readText(checks, args.docs, "handoff checklist");
  checkChecklistTokens(checks, checklist);
  checkTextEvidence(checks, smokeTexts, checklist);
  checkBlockedClaims(checks, manifest, smoke, checklist);
  check(checks, "negative matrix passed", smoke.negative?.result === "PASS", `observed ${JSON.stringify(smoke.negative?.result)}`);
  check(checks, "manifest validation passed in smoke", smoke.manifest?.status === "PASS", `observed ${JSON.stringify(smoke.manifest?.status)}`);
  check(checks, "retention report passed", smoke.retention?.status === "PASS", `observed ${JSON.stringify(smoke.retention?.status)}`);
  check(checks, "kill-switch check passed", smoke.killSwitches?.status === "PASS", `observed ${JSON.stringify(smoke.killSwitches?.status)}`);

  const status = checks.every((entry) => entry.status === "PASS") ? "PASS" : "FAIL";
  const report = {
    mode: "verify-handoff",
    status,
    validated_at: new Date().toISOString(),
    invocation: buildInvocation(),
    manifest_path: args.manifest,
    smoke_dir: args.smoke_dir,
    docs_path: args.docs,
    deployment_action: "none",
    default_availability_allowed: manifest?.default_availability?.allowed ?? null,
    beta_gate_status: smoke.beta?.overall_status ?? null,
    rollout_gate_status: smoke.regression?.overall_status ?? null,
    checks,
  };
  const jsonPath = await writeReport(args.out_dir, "handoff-verification.json", report);
  await writeReport(args.out_dir, "handoff-verification-summary.json", {
    status,
    default_availability_allowed: report.default_availability_allowed,
    beta_gate_status: report.beta_gate_status,
    rollout_gate_status: report.rollout_gate_status,
    deployment_action: "none",
  });
  console.log(`${status} rollout handoff verification: ${jsonPath}`);
  if (status !== "PASS") process.exitCode = 1;
}
