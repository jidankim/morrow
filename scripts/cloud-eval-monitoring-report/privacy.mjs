export const forbiddenFieldNames = new Set([
  "raw_text",
  "raw_prompt",
  "prompt",
  "raw_response",
  "response",
  "provider_json",
  "raw_json",
  "messages",
  "message_body",
  "full_message",
  "candidate_title",
  "raw_title",
  "title",
  "unredacted_title",
  "app_data_path",
  "appDataDir",
  "diagnostics_path",
  "diagnosticsPath",
  "local_path",
  "screenshot",
  "screenshot_path",
  "auth_token",
  "api_key",
  "secret",
  "native_id",
  "native_identifier",
  "chat_guid",
  "message_guid",
  "event_id",
  "reminder_id",
]);

const forbiddenContentPatterns = [
  { name: "privacy_canary", pattern: /MORROW_PRIVACY_CANARY_RAW|PHASE6_RAW_CONTENT_CANARY_DO_NOT_STORE/ },
  { name: "absolute_local_path", pattern: /\/(?:Users|private|var\/folders)\// },
  { name: "provider_json_content", pattern: /\bprovider_json\b/i },
  { name: "raw_prompt_content", pattern: /\braw_prompt\b/i },
  { name: "screenshot_content", pattern: /\bscreenshot\b/i },
];

function scrubDetail(detail) {
  return String(detail)
    .replaceAll("raw_text", "forbidden_raw_field")
    .replaceAll("provider_json", "forbidden_provider_payload_field")
    .replaceAll("MORROW_PRIVACY_CANARY_RAW", "forbidden_privacy_canary")
    .replaceAll("PHASE6_RAW_CONTENT_CANARY_DO_NOT_STORE", "forbidden_content_canary");
}

export function addError(errors, name, detail) {
  errors.push({ name, detail: scrubDetail(detail) });
}

export function collectForbiddenFields(node, trail, errors) {
  if (Array.isArray(node)) {
    node.forEach((item, index) => collectForbiddenFields(item, `${trail}[${index}]`, errors));
    return;
  }
  if (!node || typeof node !== "object") return;
  for (const [key, value] of Object.entries(node)) {
    const nextTrail = trail ? `${trail}.${key}` : key;
    if (forbiddenFieldNames.has(key)) addError(errors, "FORBIDDEN_FIELD", nextTrail);
    collectForbiddenFields(value, nextTrail, errors);
  }
}

export function scanForbiddenContent(text, errors) {
  for (const entry of forbiddenContentPatterns) {
    if (entry.pattern.test(text)) addError(errors, "FORBIDDEN_CONTENT", entry.name);
  }
}
