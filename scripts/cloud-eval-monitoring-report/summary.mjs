function collectDimension(summary, key, value) {
  if (typeof value === "string" && value.length > 0) summary.dimension_sets[key].add(value);
}

export function summarizeDimensions(events) {
  const summary = {
    total_events: events.length,
    evaluable_count: 0,
    audit_only_count: 0,
    deletion_suppressed_count: 0,
    quiet_low_confidence_count: 0,
    provider_failure_count: 0,
    schema_failure_count: 0,
    latency_sample_count: 0,
    error_sample_count: 0,
    label_pair_count: 0,
    dimension_sets: {
      app_versions: new Set(),
      model_versions: new Set(),
      router_versions: new Set(),
      release_ids: new Set(),
      release_channels: new Set(),
      intent_types: new Set(),
      decision_outcomes: new Set(),
      feedback_labels: new Set(),
    },
  };

  for (const event of events) {
    if (event.evaluable === true) summary.evaluable_count += 1;
    if (event.evaluable === false) summary.audit_only_count += 1;
    if (event.deletion_state === "suppressed") summary.deletion_suppressed_count += 1;
    if (event.intent_type === "quiet_low_confidence" || event.decision_outcome === "quiet") {
      summary.quiet_low_confidence_count += 1;
    }
    if (event.event_type === "provider_failure" || event.error_kind === "provider_unavailable") {
      summary.provider_failure_count += 1;
    }
    if (event.event_type === "schema_failure" || event.error_kind === "schema_validation_failed") {
      summary.schema_failure_count += 1;
    }
    if (Number.isFinite(event.latency_ms)) summary.latency_sample_count += 1;
    if (typeof event.error_kind === "string" && event.error_kind !== "none") summary.error_sample_count += 1;
    if (typeof event.expected_label === "string" && typeof event.actual_label === "string") {
      summary.label_pair_count += 1;
    }
    collectDimension(summary, "app_versions", event.app_version);
    collectDimension(summary, "model_versions", event.model_id);
    collectDimension(summary, "router_versions", event.router_version);
    collectDimension(summary, "release_ids", event.release_id);
    collectDimension(summary, "release_channels", event.release_channel);
    collectDimension(summary, "intent_types", event.intent_type);
    collectDimension(summary, "decision_outcomes", event.decision_outcome);
    collectDimension(summary, "feedback_labels", event.feedback_label);
  }

  return {
    ...summary,
    dimension_sets: Object.fromEntries(
      Object.entries(summary.dimension_sets).map(([key, set]) => [key, [...set].sort()]),
    ),
  };
}
