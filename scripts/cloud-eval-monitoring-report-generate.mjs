import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import {
  readJson,
  validateInputFixture,
  writeInputValidation,
  writeJson,
} from "./cloud-eval-monitoring-report-lib.mjs";
import { summarizeDimensions } from "./cloud-eval-monitoring-report/summary.mjs";

const sliceGroupingKeys = [
  "schema_version",
  "app_version",
  "build_id",
  "release_channel",
  "release_id",
  "model_id",
  "router_version",
  "prompt_version",
  "template_id",
  "intent_type",
  "time_bucket_utc",
];

const countKeys = [
  "event_type",
  "decision_outcome",
  "feedback_label",
  "expected_label",
  "actual_label",
  "error_kind",
  "provider_status",
  "schema_validation_status",
  "confidence_bucket",
  "retention_state",
  "deletion_state",
  "route_reason_code",
];

function sha256(text) {
  return crypto.createHash("sha256").update(text).digest("hex");
}

function increment(map, key) {
  map[key] = (map[key] ?? 0) + 1;
}

function rate(numerator, denominator) {
  return denominator > 0 ? Number((numerator / denominator).toFixed(6)) : null;
}

function numericSummary(values) {
  if (values.length === 0) return { sample_count: 0, min_ms: null, max_ms: null, average_ms: null };
  const sum = values.reduce((total, value) => total + value, 0);
  return {
    sample_count: values.length,
    min_ms: Math.min(...values),
    max_ms: Math.max(...values),
    average_ms: Number((sum / values.length).toFixed(3)),
  };
}

function blankSlice(grouping) {
  return {
    grouping,
    audit_totals: {
      total_records: 0,
      audit_only_records: 0,
      deletion_suppressed_records: 0,
    },
    evaluated_counts: {
      evaluable_records: 0,
      label_pair_records: 0,
      label_match_records: 0,
      precision_numerator: 0,
      precision_denominator: 0,
      recall_numerator: 0,
      recall_denominator: 0,
      precision_rate: null,
      recall_rate: null,
    },
    decision_totals: {},
    feedback_totals: {},
    timing_totals: { sample_count: 0, min_ms: null, max_ms: null, average_ms: null },
    error_totals: { error_records: 0, by_error_kind: {}, provider_status: {}, schema_validation_status: {} },
    sanitized_cluster_dimensions: {},
    internal_latency_samples: [],
  };
}

function groupingFor(event) {
  return Object.fromEntries(sliceGroupingKeys.map((key) => [key, event[key]]));
}

function groupingKey(grouping) {
  return JSON.stringify(sliceGroupingKeys.map((key) => grouping[key]));
}

function isAuditOnly(event) {
  return event.evaluable !== true || event.deletion_state === "suppressed" || event.event_type === "deletion_audit";
}

function addEvent(slice, event) {
  slice.audit_totals.total_records += 1;

  if (isAuditOnly(event)) {
    slice.audit_totals.audit_only_records += 1;
    if (event.deletion_state === "suppressed") slice.audit_totals.deletion_suppressed_records += 1;
    return;
  }

  for (const key of countKeys) increment(slice.sanitized_cluster_dimensions, `${key}:${event[key]}`);
  slice.evaluated_counts.evaluable_records += 1;
  increment(slice.decision_totals, event.decision_outcome);
  increment(slice.feedback_totals, event.feedback_label);
  increment(slice.error_totals.by_error_kind, event.error_kind);
  increment(slice.error_totals.provider_status, event.provider_status);
  increment(slice.error_totals.schema_validation_status, event.schema_validation_status);
  if (event.error_kind !== "none") slice.error_totals.error_records += 1;
  if (Number.isInteger(event.latency_ms)) slice.internal_latency_samples.push(event.latency_ms);
  if (typeof event.expected_label === "string" && typeof event.actual_label === "string") {
    slice.evaluated_counts.label_pair_records += 1;
    slice.evaluated_counts.precision_denominator += 1;
    slice.evaluated_counts.recall_denominator += 1;
    if (event.expected_label === event.actual_label) {
      slice.evaluated_counts.label_match_records += 1;
      slice.evaluated_counts.precision_numerator += 1;
      slice.evaluated_counts.recall_numerator += 1;
    }
  }
}

function finalizeSlice(slice) {
  const counts = slice.evaluated_counts;
  counts.precision_rate = rate(counts.precision_numerator, counts.precision_denominator);
  counts.recall_rate = rate(counts.recall_numerator, counts.recall_denominator);
  slice.timing_totals = numericSummary(slice.internal_latency_samples);
  delete slice.internal_latency_samples;
  slice.sanitized_cluster_dimensions = Object.fromEntries(
    Object.entries(slice.sanitized_cluster_dimensions).sort(([left], [right]) => left.localeCompare(right)),
  );
  return slice;
}

function aggregate(events) {
  const byGroup = new Map();
  const overall = blankSlice({ scope: "all_validated_events" });
  for (const event of events) {
    const grouping = groupingFor(event);
    const key = groupingKey(grouping);
    if (!byGroup.has(key)) byGroup.set(key, blankSlice(grouping));
    addEvent(byGroup.get(key), event);
    addEvent(overall, event);
  }
  const slices = [...byGroup.values()]
    .map(finalizeSlice)
    .sort((left, right) => groupingKey(left.grouping).localeCompare(groupingKey(right.grouping)));
  return { slices, overall: finalizeSlice(overall) };
}

function jobForSlice(slice) {
  const stableKey = groupingKey(slice.grouping);
  return {
    job_id: `eval-job-${sha256(stableKey).slice(0, 16)}`,
    grouping: slice.grouping,
    source_record_counts: {
      total_records: slice.audit_totals.total_records,
      evaluable_records: slice.evaluated_counts.evaluable_records,
      audit_only_records: slice.audit_totals.audit_only_records,
      deletion_suppressed_records: slice.audit_totals.deletion_suppressed_records,
    },
  };
}

export function generate(args) {
  const validationResult = validateInputFixture(args.input);
  if (validationResult.validation) writeInputValidation(args.outDir, validationResult.validation);
  if (validationResult.status !== "PASS") return { status: "FAIL", errors: validationResult.errors };

  const root = readJson(args.input);
  const inputHash = sha256(fs.readFileSync(args.input, "utf8"));
  const { slices, overall } = aggregate(root.events);
  const common = {
    generated_at_utc: root.generated_at_utc,
    source_fixture_id: root.fixture_id,
    source_payload_sha256: inputHash,
    grouping_keys: sliceGroupingKeys,
    deletion_retention_semantics: {
      deletion_suppressed_records: "audit_totals_only",
      precision_recall_denominators: "exclude_deletion_suppressed_and_audit_only_records",
    },
  };

  writeJson(path.join(args.outDir, "eval-job-input.json"), {
    schema_version: "phase6_cloud_eval_monitoring_eval_job_input_v1",
    ...common,
    jobs: slices.map(jobForSlice),
  });
  writeJson(path.join(args.outDir, "eval-slices.json"), {
    schema_version: "phase6_cloud_eval_monitoring_eval_slices_v1",
    ...common,
    slices,
  });
  writeJson(path.join(args.outDir, "aggregation-summary.json"), {
    schema_version: "phase6_cloud_eval_monitoring_aggregation_summary_v1",
    ...common,
    artifact_paths: {
      eval_job_input: "eval-job-input.json",
      eval_slices: "eval-slices.json",
      input_validation: "input-validation.json",
    },
    dimensions: summarizeDimensions(root.events),
    overall_counts: overall,
    slice_count: slices.length,
    false_positive_clustering_policy: {
      status: "deferred_to_metrics_phase",
      allowed_sanitized_dimensions: [
        "route_reason_code",
        "decision_outcome",
        "intent_type",
        "model_id",
        "router_version",
        "error_kind",
      ],
      content_clustering: "not_derived",
    },
  });
  return { status: "PASS", errors: [], outDir: args.outDir };
}
