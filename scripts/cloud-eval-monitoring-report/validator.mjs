import fs from "node:fs";
import {
  allowedEventFields,
  allowedFreshnessFields,
  allowedRootFields,
  enumValues,
  eventSchemaVersion,
  fixtureSchemaVersion,
  groupingKeys,
  requiredEventFields,
  sanitizedRootStringRules,
  sanitizedStringRules,
} from "./schema.mjs";
import { addError, collectForbiddenFields, scanForbiddenContent } from "./privacy.mjs";
import { summarizeDimensions } from "./summary.mjs";

function parseTimestamp(value) {
  if (typeof value !== "string") return Number.NaN;
  return Date.parse(value);
}

function requireString(record, key, errors, index) {
  const value = record[key];
  if (typeof value !== "string" || value.length === 0) {
    addError(errors, "MISSING_REQUIRED_FIELD", `events[${index}].${key}`);
    return "";
  }
  return value;
}

function requireGroupingKeys(record, errors, index) {
  for (const key of groupingKeys) {
    if (typeof record[key] !== "string" || record[key].length === 0) {
      addError(errors, "MISSING_GROUPING_KEY", `events[${index}].${key}`);
    }
  }
}

function validateObjectFields(value, allowedFields, label, errors) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    addError(errors, "MALFORMED_SCHEMA", `${label} must be an object`);
    return false;
  }
  for (const key of Object.keys(value)) {
    if (!allowedFields.has(key)) addError(errors, "UNKNOWN_FIELD", `${label}.${key}`);
  }
  return true;
}

function validateEnum(record, key, errors, index) {
  const value = record[key];
  const allowed = enumValues[key];
  if (typeof value === "string" && allowed && !allowed.has(value)) {
    addError(errors, "INVALID_ENUM_VALUE", `events[${index}].${key}:${value}`);
  }
}

function validateSanitizedString(record, key, errors, index) {
  const rule = sanitizedStringRules[key];
  const value = record[key];
  if (!rule || typeof value !== "string" || value.length === 0) return;
  if (rule.timestamp) {
    if (!Number.isFinite(parseTimestamp(value))) {
      addError(errors, "SANITIZED_TOKEN_EXPECTED", `events[${index}].${key}:${rule.label}`);
    }
    return;
  }
  if (!rule.pattern.test(value)) {
    addError(errors, "SANITIZED_TOKEN_EXPECTED", `events[${index}].${key}:${rule.label}`);
  }
}

function validateSanitizedStrings(record, errors, index) {
  for (const key of Object.keys(sanitizedStringRules)) validateSanitizedString(record, key, errors, index);
}

function validateSanitizedRootString(root, key, errors) {
  const rule = sanitizedRootStringRules[key];
  const value = root[key];
  if (typeof value !== "string" || value.length === 0) {
    addError(errors, "SANITIZED_TOKEN_EXPECTED", `root.${key}:${rule.label}`);
    return;
  }
  if (!rule.pattern.test(value)) {
    addError(errors, "SANITIZED_TOKEN_EXPECTED", `root.${key}:${rule.label}`);
  }
}

function validateSanitizedRootStrings(root, errors) {
  for (const key of Object.keys(sanitizedRootStringRules)) validateSanitizedRootString(root, key, errors);
}

function safeFixtureId(root) {
  const value = root.fixture_id;
  const rule = sanitizedRootStringRules.fixture_id;
  return typeof value === "string" && rule.pattern.test(value) ? value : "invalid_fixture_id";
}

function validateFreshness(root, events, errors) {
  const generatedAt = parseTimestamp(root.generated_at_utc);
  if (!Number.isFinite(generatedAt)) {
    addError(errors, "MALFORMED_TIMESTAMP", "generated_at_utc");
    return;
  }
  const maxAgeHours = root.freshness?.max_event_age_hours ?? 168;
  if (!Number.isInteger(maxAgeHours) || maxAgeHours < 1 || maxAgeHours > 2160) {
    addError(errors, "MALFORMED_SCHEMA", "freshness.max_event_age_hours");
    return;
  }
  const maxAgeMs = maxAgeHours * 60 * 60 * 1000;
  for (let index = 0; index < events.length; index += 1) {
    const eventAt = parseTimestamp(events[index].event_timestamp_utc);
    if (!Number.isFinite(eventAt)) {
      addError(errors, "MALFORMED_TIMESTAMP", `events[${index}].event_timestamp_utc`);
    } else if (generatedAt - eventAt > maxAgeMs || eventAt - generatedAt > 60 * 60 * 1000) {
      addError(errors, "STALE_DATA", `events[${index}].event_timestamp_utc`);
    }
  }
}

function validateDeletionSemantics(record, errors, index) {
  if (record.deletion_state === "suppressed") {
    if (record.event_type !== "deletion_audit" || record.evaluable !== false) {
      addError(errors, "DELETION_SUPPRESSED_SEMANTICS", `events[${index}]`);
    }
    for (const key of ["deletion_audit_id", "deletion_requested_at_utc", "deletion_effective_at_utc", "suppression_reason"]) {
      requireString(record, key, errors, index);
    }
    return;
  }
  if (record.event_type === "deletion_audit") addError(errors, "DELETION_SUPPRESSED_SEMANTICS", `events[${index}]`);
}

function validateEvent(record, errors, index) {
  validateObjectFields(record, allowedEventFields, `events[${index}]`, errors);
  for (const key of requiredEventFields) requireString(record, key, errors, index);
  requireGroupingKeys(record, errors, index);
  for (const key of Object.keys(enumValues)) validateEnum(record, key, errors, index);
  validateSanitizedStrings(record, errors, index);
  if (record.schema_version !== eventSchemaVersion) {
    addError(errors, "MALFORMED_SCHEMA_VERSION", `events[${index}].schema_version`);
  }
  if (!Object.hasOwn(record, "latency_ms")) {
    addError(errors, "MISSING_REQUIRED_FIELD", `events[${index}].latency_ms`);
  }
  if (!Number.isInteger(record.latency_ms) || record.latency_ms < 0) {
    addError(errors, "MALFORMED_TIMING_METADATA", `events[${index}].latency_ms`);
  }
  if (!Object.hasOwn(record, "evaluable")) {
    addError(errors, "MISSING_REQUIRED_FIELD", `events[${index}].evaluable`);
  }
  if (typeof record.evaluable !== "boolean") addError(errors, "MALFORMED_SCHEMA", `events[${index}].evaluable`);
  validateDeletionSemantics(record, errors, index);
}

function buildValidation(root, events, errors) {
  const dimensions = summarizeDimensions(events);
  return {
    schema_version: "phase6_cloud_eval_monitoring_input_validation_v1",
    status: errors.length === 0 ? "PASS" : "FAIL",
    fixture_id: safeFixtureId(root),
    generated_at_utc: root.generated_at_utc,
    accepted_schema_version: fixtureSchemaVersion,
    event_schema_version: eventSchemaVersion,
    dimensions,
    deletion_suppressed_semantics: {
      behavior: "valid_audit_only_excluded_from_evaluable_counts",
      deletion_suppressed_count: dimensions.deletion_suppressed_count,
      audit_only_count: dimensions.audit_only_count,
      evaluable_count: dimensions.evaluable_count,
    },
    sanitizer: {
      allowed_root_fields: [...allowedRootFields].sort(),
      allowed_event_fields: [...allowedEventFields].sort(),
      allowed_string_shapes: Object.fromEntries(
        [...Object.entries(sanitizedRootStringRules), ...Object.entries(sanitizedStringRules)].map(([key, rule]) => [key, rule.label]),
      ),
      forbidden_error_names: ["FORBIDDEN_FIELD", "FORBIDDEN_CONTENT", "SANITIZED_TOKEN_EXPECTED"],
    },
    errors,
  };
}

export function validateInputFixture(filePath) {
  const errors = [];
  const text = fs.readFileSync(filePath, "utf8");
  scanForbiddenContent(text, errors);
  let root;
  try {
    root = JSON.parse(text);
  } catch (error) {
    addError(errors, "MALFORMED_JSON", error.message);
    return { status: "FAIL", errors, validation: null };
  }
  collectForbiddenFields(root, "", errors);
  if (!validateObjectFields(root, allowedRootFields, "root", errors)) {
    const validation = buildValidation({}, [], errors);
    return { status: validation.status, errors, validation };
  }
  validateSanitizedRootStrings(root, errors);
  if (root.schema_version !== fixtureSchemaVersion) addError(errors, "MALFORMED_SCHEMA_VERSION", "root.schema_version");
  if (root.freshness !== undefined) validateObjectFields(root.freshness, allowedFreshnessFields, "root.freshness", errors);
  const events = Array.isArray(root.events) ? root.events : [];
  if (!Array.isArray(root.events) || events.length === 0) addError(errors, "MALFORMED_SCHEMA", "events must be a non-empty array");
  events.forEach((event, index) => validateEvent(event, errors, index));
  validateFreshness(root, events, errors);
  const validation = buildValidation(root, events, errors);
  return { status: validation.status, errors, validation };
}
