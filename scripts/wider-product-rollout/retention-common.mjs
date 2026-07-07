import fs from "node:fs";
import path from "node:path";

export const usage =
  "Usage: node scripts/wider-product-rollout-retention-qa.mjs --out-dir <path> [--fixture <path>]";
export const fixedNowSeconds = 1_783_000_000;
export const daySeconds = 24 * 60 * 60;
export const defaultRetentionDays = 30;
export const runtimeDiagnosticsDirs = ["diagnostics/traces", "diagnostics/evals", "diagnostics/exports"];
export const planningEvidencePrefix = ".omo/evidence/";
export const forbiddenEvidencePatterns = [
  ["openai_secret_token", /\bsk-[A-Za-z0-9]{8,}\b/],
  ["raw_prompt_field", /"prompt"\s*:/],
  ["raw_response_field", /"response"\s*:/],
  ["raw_text_field", /"raw_text"\s*:/],
  ["provider_json_field", /"provider_json"\s*:/],
  ["embedding_field", /"embedding"\s*:/],
];
const evidenceReplacements = [
  [/app-data\/provider-credentials\/morrow-openai-provider-api-key[.]marker/g, "<morrow-provider-marker-path>"],
  [/morrow-openai-provider-api-key/g, "<morrow-provider-token-kind>"],
  [/provider-credentials/g, "<morrow-provider-marker-dir>"],
];

export class QaError extends Error {
  constructor(message) {
    super(message);
    this.name = "QaError";
  }
}

export function parseArgs(argv) {
  const parsed = { outDir: undefined, fixture: undefined };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--out-dir") {
      parsed.outDir = argv[index + 1];
      index += 1;
    } else if (arg === "--fixture") {
      parsed.fixture = argv[index + 1];
      index += 1;
    } else {
      throw new QaError(`${usage}\nUnexpected argument: ${arg}`);
    }
  }
  if (parsed.outDir === undefined) {
    throw new QaError(usage);
  }
  return parsed;
}

export function normalizeRelative(value) {
  return value.split(path.sep).join("/");
}

export function readText(repoRoot, relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), "utf8");
}

export function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(`${filePath}.tmp`, `${JSON.stringify(sanitizeEvidenceValue(value), null, 2)}\n`);
  fs.renameSync(`${filePath}.tmp`, filePath);
}

export function writeText(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(`${filePath}.tmp`, sanitizeEvidenceString(value));
  fs.renameSync(`${filePath}.tmp`, filePath);
}

export function addCheck(checks, status, id, scenario, observable, artifact, details = {}) {
  checks.push({
    status,
    id,
    scenario,
    observable: sanitizeEvidenceString(observable),
    artifact: sanitizeEvidenceString(artifact),
    details: sanitizeEvidenceValue(details),
  });
}

export function expectIncludes(checks, id, relativePath, text, needles, scenario) {
  const missing = needles.filter((needle) => !text.includes(needle));
  addCheck(
    checks,
    missing.length === 0 ? "PASS" : "FAIL",
    id,
    scenario,
    missing.length === 0 ? `all ${needles.length} required source fragments found` : `missing: ${missing.join(", ")}`,
    relativePath,
    { needles }
  );
}

export function retentionDaysIsAccepted(value) {
  return Number.isInteger(value) && value >= 1 && value <= 365;
}

export function applyRetention(tracesDir, retentionDays, nowSeconds) {
  const removed = [];
  for (const dirent of fs.readdirSync(tracesDir, { withFileTypes: true })) {
    if (!dirent.isFile()) {
      continue;
    }
    const match = /^trace-(\d+)-\d+[.]jsonl$/.exec(dirent.name);
    const createdAt = match === null ? undefined : Number.parseInt(match[1], 10);
    if (createdAt === undefined) {
      continue;
    }
    if (nowSeconds - createdAt > retentionDays * daySeconds) {
      const filePath = path.join(tracesDir, dirent.name);
      fs.rmSync(filePath);
      removed.push(normalizeRelative(path.relative(tracesDir, filePath)));
    }
  }
  return removed.sort();
}

export function listFiles(root) {
  if (!fs.existsSync(root)) {
    return [];
  }
  const results = [];
  const visit = (current) => {
    for (const dirent of fs.readdirSync(current, { withFileTypes: true })) {
      const absolutePath = path.join(current, dirent.name);
      if (dirent.isDirectory()) {
        visit(absolutePath);
      } else if (dirent.isFile()) {
        results.push(normalizeRelative(path.relative(root, absolutePath)));
      }
    }
  };
  visit(root);
  return results.sort();
}

export function scanPlanningEvidence(filePath) {
  const text = fs.readFileSync(filePath, "utf8");
  return forbiddenEvidencePatterns.filter(([_name, pattern]) => pattern.test(text)).map(([name]) => name);
}

export function sanitizeEvidenceString(value) {
  return evidenceReplacements.reduce((current, [pattern, replacement]) => current.replace(pattern, replacement), value);
}

export function sanitizeEvidenceValue(value) {
  if (typeof value === "string") {
    return sanitizeEvidenceString(value);
  }
  if (Array.isArray(value)) {
    return value.map((item) => sanitizeEvidenceValue(item));
  }
  if (value !== null && typeof value === "object") {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, sanitizeEvidenceValue(item)]));
  }
  return value;
}
