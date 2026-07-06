import fs from "node:fs";

export const reportName = "trajectory-report.json";

export const requiredCaseFamilies = {
  scheduled_meeting_accepted: {
    caseId: "phase5:scheduled_meeting_accepted:v1",
    requiredOutcome: "proposal_accepted",
  },
  scheduled_meeting_rejected: {
    caseId: "phase5:scheduled_meeting_rejected:v1",
    requiredOutcome: "proposal_rejected",
  },
  scheduled_meeting_edited_before_approval: {
    caseId: "phase5:scheduled_meeting_edited_before_approval:v1",
    requiredOutcome: "proposal_edited_before_approval",
  },
  task_reminder_accepted: {
    caseId: "phase5:task_reminder_accepted:v1",
    requiredOutcome: "proposal_accepted",
  },
  task_reminder_rejected: {
    caseId: "phase5:task_reminder_rejected:v1",
    requiredOutcome: "proposal_rejected",
  },
  provider_quiet_low_confidence: {
    caseId: "phase5:provider_quiet_low_confidence:v1",
    requiredOutcome: "quiet_low_confidence",
  },
  collateral_damage_non_target_preserved: {
    caseId: "phase5:collateral_damage_non_target_preserved:v1",
    requiredOutcome: "non_target_preserved",
  },
  replay_idempotent_retry: {
    caseId: "phase5:replay_idempotent_retry:v1",
    requiredOutcome: "replay_idempotent",
  },
  privacy_canary_rejection: {
    caseId: "phase5:privacy_canary_rejection:v1",
    requiredOutcome: "privacy_canary_rejected",
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
  "local_path",
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

export function assertNoForbiddenContent(filePath) {
  const text = fs.readFileSync(filePath, "utf8");
  const forbiddenPatterns = [
    /PHASE5_RAW_MESSAGE_PROVIDER_CANARY_DO_NOT_STORE/,
    /MORROW_PRIVACY_CANARY_RAW_TRAJECTORY/,
    /\/Users\//,
    /\/private\//,
    /(^|[{\s,"])raw_text"?\s*:/m,
    /(^|[{\s,"])prompt"?\s*:/m,
    /(^|[{\s,"])response"?\s*:/m,
    /(^|[{\s,"])raw_json"?\s*:/m,
    /(^|[{\s,"])provider_json"?\s*:/m,
    /(^|[{\s,"])full_message"?\s*:/m,
    /(^|[{\s,"])raw_title"?\s*:/m,
    /(^|[{\s,"])title_text"?\s*:/m,
    /(^|[{\s,"])unredacted_title"?\s*:/m,
    /(^|[{\s,"])chat_guid"?\s*:/m,
    /(^|[{\s,"])anchor_message_guid"?\s*:/m,
    /(^|[{\s,"])native_identifier"?\s*:/m,
    /(^|[{\s,"])nativeIdentifier"?\s*:/m,
    /(^|[{\s,"])appDataDir"?\s*:/m,
    /(^|[{\s,"])diagnosticsPath"?\s*:/m,
    /(^|[{\s,"])local_path"?\s*:/m,
  ];
  for (const pattern of forbiddenPatterns) {
    if (pattern.test(text)) die(`forbidden raw content in ${filePath}: ${pattern}`);
  }
}

export function assertCleanArtifact(filePath) {
  assertNoForbiddenContent(filePath);
  if (filePath.endsWith(".json")) assertNoForbiddenFields(filePath);
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

export function familyFromCaseId(caseId) {
  const match = /^phase5:([^:]+):v1$/.exec(caseId);
  return match ? match[1] : "";
}
