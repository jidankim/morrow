import fs from "node:fs";

export const correctionTrace = {
  component: "correction",
  operation: "user_correction",
  decision: "user_corrected",
  outcome: "noop",
  reasonCode: "user_correction_applied",
  labelType: "field_quality",
  labelValue: "title_edited",
};

export const requiredOutcomeCoverage = {
  accepted: {
    labelType: "proposal_outcome",
    labelValue: "accepted",
    artifact: "command-logs/cargo-reconcile-human-approval-correction.txt",
    observable:
      "PASS named assertions: accepted rejected_observed pending_edited title_edited time_edited unknown failed_external_creation no overwrite",
  },
  rejected_observed: {
    labelType: "proposal_outcome",
    labelValue: "rejected_observed",
    artifact: "command-logs/cargo-reconcile-human-approval-correction.txt",
    observable:
      "PASS named assertions: accepted rejected_observed pending_edited title_edited time_edited unknown failed_external_creation no overwrite",
  },
  pending_edited: {
    labelType: "proposal_outcome",
    labelValue: "pending_edited",
    artifact: "command-logs/cargo-reconcile-human-approval-correction.txt",
    observable:
      "PASS named assertions: accepted rejected_observed pending_edited title_edited time_edited unknown failed_external_creation no overwrite",
  },
  unknown: {
    labelType: "proposal_outcome",
    labelValue: "unknown",
    artifact: "command-logs/cargo-reconcile-human-approval-correction.txt",
    observable:
      "PASS named assertions: accepted rejected_observed pending_edited title_edited time_edited unknown failed_external_creation no overwrite",
  },
};

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
  "native_identifier",
  "nativeIdentifier",
  "appDataDir",
  "diagnosticsPath",
]);

export function die(message) {
  console.error(`error: ${message}`);
  process.exit(64);
}

export function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

export function writeJson(filePath, value) {
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`);
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
  const hits = [];
  collectForbiddenFields(readJson(filePath), "", hits);
  if (hits.length > 0) die(`forbidden fields in ${filePath}: ${hits.join(", ")}`);
}

export function cargoPassedCount(logText) {
  const matches = logText.matchAll(/test result:\s+ok\.\s+(\d+)\s+passed;/g);
  let sawCargoTestResult = false;
  let passed = 0;
  for (const match of matches) {
    sawCargoTestResult = true;
    passed += Number.parseInt(match[1], 10);
  }
  return { passed, sawCargoTestResult };
}

export function vitestPassedCount(logText) {
  const match = logText.match(/Tests\s+(\d+)\s+passed/);
  return match ? Number.parseInt(match[1], 10) : 0;
}
