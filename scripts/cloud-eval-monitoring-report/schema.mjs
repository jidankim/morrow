export const fixtureSchemaVersion = "phase6_cloud_eval_monitoring_events_v1";
export const eventSchemaVersion = "phase6_cloud_eval_monitoring_event_v1";

export const allowedRootFields = new Set(["schema_version", "fixture_id", "generated_at_utc", "freshness", "events"]);
export const allowedFreshnessFields = new Set(["max_event_age_hours"]);

export const allowedEventFields = new Set([
  "event_type",
  "schema_version",
  "event_timestamp_utc",
  "time_bucket_utc",
  "app_version",
  "build_id",
  "release_channel",
  "release_id",
  "model_id",
  "router_version",
  "prompt_version",
  "template_id",
  "feature_snapshot_id",
  "intent_type",
  "decision_outcome",
  "feedback_label",
  "expected_label",
  "actual_label",
  "confidence_bucket",
  "latency_ms",
  "error_kind",
  "provider_status",
  "schema_validation_status",
  "route_reason_code",
  "installation_surrogate",
  "user_surrogate",
  "retention_state",
  "deletion_state",
  "deletion_audit_id",
  "deletion_requested_at_utc",
  "deletion_effective_at_utc",
  "suppression_reason",
  "evaluable",
]);

export const groupingKeys = [
  "app_version",
  "build_id",
  "release_channel",
  "release_id",
  "model_id",
  "router_version",
  "prompt_version",
  "template_id",
  "feature_snapshot_id",
  "intent_type",
  "time_bucket_utc",
];

export const requiredEventFields = [
  "event_type",
  "schema_version",
  "event_timestamp_utc",
  "decision_outcome",
  "feedback_label",
  "expected_label",
  "actual_label",
  "confidence_bucket",
  "error_kind",
  "provider_status",
  "schema_validation_status",
  "route_reason_code",
  "retention_state",
  "deletion_state",
  ...groupingKeys,
];

const labelValues = ["accepted", "rejected", "edited", "quiet", "failed", "suppressed"];

export const enumValues = {
  event_type: new Set(["eval_decision", "provider_failure", "schema_failure", "deletion_audit"]),
  release_channel: new Set(["alpha", "beta", "stable", "internal"]),
  intent_type: new Set(["calendar_event", "task_reminder", "quiet_low_confidence"]),
  decision_outcome: new Set(["accepted", "rejected", "edited", "quiet", "provider_failed", "schema_failed", "deletion_suppressed"]),
  feedback_label: new Set(labelValues),
  expected_label: new Set(labelValues),
  actual_label: new Set(labelValues),
  confidence_bucket: new Set(["high", "medium", "low", "quiet"]),
  error_kind: new Set(["none", "provider_unavailable", "provider_schema_rejected", "schema_validation_failed"]),
  provider_status: new Set(["ok", "unavailable", "schema_rejected", "not_called"]),
  schema_validation_status: new Set(["ok", "failed"]),
  retention_state: new Set(["retained", "expired", "deleted"]),
  deletion_state: new Set(["not_requested", "requested", "suppressed"]),
};

export const sanitizedStringRules = {
  schema_version: { pattern: /^phase6_cloud_eval_monitoring_event_v1$/, label: "event schema version" },
  event_timestamp_utc: { timestamp: true, label: "event timestamp" },
  time_bucket_utc: { timestamp: true, label: "time bucket" },
  app_version: { pattern: /^\d+\.\d+\.\d+(?:[+-][a-z0-9][a-z0-9.-]*)?$/, label: "semantic app version" },
  build_id: { pattern: /^build-\d{4}-\d{2}-\d{2}-[a-z0-9]+$/, label: "build id" },
  release_id: { pattern: /^rel-\d{4}-\d{2}-[a-z0-9]+(?:-[a-z0-9]+)?$/, label: "release id" },
  model_id: { pattern: /^model-[a-z0-9]+(?:-[a-z0-9]+)*$/, label: "model id" },
  router_version: { pattern: /^router-v\d+(?:\.\d+)*$/, label: "router version" },
  prompt_version: { pattern: /^prompt-v\d+(?:\.\d+)*$/, label: "prompt version" },
  template_id: { pattern: /^template-[a-z0-9]+(?:-[a-z0-9]+)*-v\d+$/, label: "template id" },
  feature_snapshot_id: { pattern: /^(?:baseline-)?snapshot-[a-z0-9]+(?:-[a-z0-9]+)*$/, label: "feature snapshot id" },
  route_reason_code: { pattern: /^[a-z][a-z0-9_]{2,63}$/, label: "route reason code" },
  installation_surrogate: { pattern: /^install-hash-[a-z0-9]+(?:-[a-z0-9]+)*$/, label: "installation surrogate" },
  user_surrogate: { pattern: /^user-hash-[a-z0-9]+(?:-[a-z0-9]+)*$/, label: "user surrogate" },
  deletion_audit_id: { pattern: /^delete-audit-hash-[a-z0-9]+(?:-[a-z0-9]+)*$/, label: "deletion audit id" },
  deletion_requested_at_utc: { timestamp: true, label: "deletion requested timestamp" },
  deletion_effective_at_utc: { timestamp: true, label: "deletion effective timestamp" },
  suppression_reason: { pattern: /^[a-z][a-z0-9_]{2,63}$/, label: "suppression reason code" },
};

export const sanitizedRootStringRules = {
  fixture_id: { pattern: /^phase6_[a-z0-9]+(?:_[a-z0-9]+)*_v\d+$/, label: "phase 6 fixture id" },
};
