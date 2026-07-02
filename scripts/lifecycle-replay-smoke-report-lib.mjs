import fs from "node:fs";

export const operations = [
  ["candidate_superseded", "lifecycle", "superseded"],
  ["candidate_rescheduled", "lifecycle", "rescheduled"],
  ["candidate_cancelled", "lifecycle", "cancelled"],
  ["calendar_dry_run", "calendar", "dry_run"],
  ["calendar_commit_idempotency", "calendar", "commit_idempotent"],
  ["replay_run", "replay", "replay_recorded"],
];

const forbiddenFields = new Set([
  "raw_text",
  "prompt",
  "response",
  "raw_json",
  "provider_json",
  "full_message",
  "raw_title",
  "title_text",
  "unredacted_title",
  "chat_guid",
  "anchor_message_guid",
  "appDataDir",
  "diagnosticsPath",
]);

export function die(message) {
  console.error(`error: ${message}`);
  process.exit(64);
}

export function writeJson(filePath, value) {
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

export function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

export function requireNonEmpty(filePath) {
  const stat = fs.existsSync(filePath) ? fs.statSync(filePath) : null;
  if (!stat || stat.size === 0) die(`expected non-empty artifact: ${filePath}`);
}

function collectForbiddenFields(node, trail, hits) {
  if (Array.isArray(node)) {
    node.forEach((item, index) => collectForbiddenFields(item, `${trail}[${index}]`, hits));
    return;
  }
  if (node && typeof node === "object") {
    for (const [key, value] of Object.entries(node)) {
      const nextTrail = trail ? `${trail}.${key}` : key;
      if (forbiddenFields.has(key)) hits.push(nextTrail);
      collectForbiddenFields(value, nextTrail, hits);
    }
  }
}

export function assertNoForbiddenFields(filePath) {
  const value = readJson(filePath);
  const hits = [];
  collectForbiddenFields(value, "", hits);
  if (hits.length > 0) die(`forbidden fields in ${filePath}: ${hits.join(", ")}`);
}
