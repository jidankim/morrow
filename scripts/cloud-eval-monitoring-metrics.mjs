import path from "node:path";
import {
  readJson,
  validateInputFixture,
  writeInputValidation,
  writeJson,
} from "./cloud-eval-monitoring-report-lib.mjs";
import { summarizeDimensions } from "./cloud-eval-monitoring-report/summary.mjs";

const thresholds = {
  schema_version: "phase6_cloud_eval_monitoring_thresholds_v1",
  thresholds_version: "2026-07-07.todo4.defaults.v1",
  tunable_defaults: {
    minimum_label_denominator: 1,
    minimum_rate_denominator: 1,
    precision_warn_below: 0.9,
    recall_warn_below: 0.9,
    latency_p95_regression_warn_ratio: 1.25,
    error_rate_regression_warn_delta: 0.05,
    false_positive_cluster_warn_count: 2,
  },
};

const labels = ["accepted", "rejected", "edited", "quiet", "failed", "suppressed"];
const sliceKeys = ["app_version", "model_id", "router_version", "release_id"];
const falsePositiveKeys = [
  "intent_type",
  "route_reason_code",
  "expected_label",
  "actual_label",
  "model_id",
  "router_version",
  "error_kind",
];

function metric(numerator, denominator, min = 1) {
  if (denominator < min) return { status: "insufficient_data", value: null, numerator, denominator };
  return { status: "computed", value: Number((numerator / denominator).toFixed(6)), numerator, denominator };
}

function percentile(values, percentileValue) {
  if (values.length === 0) return null;
  const sorted = [...values].sort((left, right) => left - right);
  const index = Math.min(sorted.length - 1, Math.ceil((percentileValue / 100) * sorted.length) - 1);
  return sorted[index];
}

function keyFor(event, keys) {
  return keys.map((key) => event[key]).join("|");
}

function evaluated(events) {
  return events.filter((event) => event.evaluable === true && event.deletion_state !== "suppressed");
}

function groupBy(events, keys) {
  const groups = new Map();
  for (const event of events) {
    const groupKey = keyFor(event, keys);
    if (!groups.has(groupKey)) {
      groups.set(groupKey, { grouping: Object.fromEntries(keys.map((key) => [key, event[key]])), events: [] });
    }
    groups.get(groupKey).events.push(event);
  }
  return [...groups.values()].sort((left, right) => JSON.stringify(left.grouping).localeCompare(JSON.stringify(right.grouping)));
}

function labelMetrics(events) {
  let matches = 0;
  let pairs = 0;
  for (const event of evaluated(events)) {
    if (typeof event.expected_label !== "string" || typeof event.actual_label !== "string") continue;
    pairs += 1;
    if (event.expected_label === event.actual_label) matches += 1;
  }
  return {
    precision: metric(matches, pairs, thresholds.tunable_defaults.minimum_label_denominator),
    recall: metric(matches, pairs, thresholds.tunable_defaults.minimum_label_denominator),
  };
}

function labelFamily(events, keys) {
  return groupBy(events, keys).map((group) => ({ ...group.grouping, ...labelMetrics(group.events) }));
}

function outcomeRates(events) {
  const rows = {};
  const usable = evaluated(events);
  for (const [outcome, rateName] of [
    ["accepted", "approval_rate"],
    ["rejected", "rejection_rate"],
    ["edited", "edit_rate"],
  ]) {
    const count = usable.filter((event) => event.decision_outcome === outcome).length;
    rows[rateName] = metric(count, usable.length, thresholds.tunable_defaults.minimum_rate_denominator);
  }
  return rows;
}

function latencyStats(events) {
  const values = evaluated(events)
    .map((event) => event.latency_ms)
    .filter((value) => Number.isInteger(value));
  return {
    status: values.length === 0 ? "insufficient_data" : "computed",
    sample_count: values.length,
    p50_ms: percentile(values, 50),
    p95_ms: percentile(values, 95),
  };
}

function errorRates(events) {
  const usable = evaluated(events);
  const kinds = [...new Set([...usable.map((event) => event.error_kind), "provider_unavailable", "provider_schema_rejected", "schema_validation_failed"])]
    .filter((kind) => kind && kind !== "none")
    .sort();
  return Object.fromEntries(
    kinds.map((kind) => [
      kind,
      metric(usable.filter((event) => event.error_kind === kind).length, usable.length, thresholds.tunable_defaults.minimum_rate_denominator),
    ]),
  );
}

function currentAndGrouped(events) {
  return {
    overall: {
      label_quality: labelMetrics(events),
      outcome_rates: outcomeRates(events),
      latency: latencyStats(events),
      error_rates: errorRates(events),
    },
    by_intent_type: labelFamily(events, ["intent_type"]),
    by_app_model_router_release: labelFamily(events, sliceKeys),
  };
}

function regressionRows(currentEvents, baselineEvents, key, valueSelector) {
  return groupBy(currentEvents, [key]).map((currentGroup) => {
    const baselineGroup = groupBy(baselineEvents, [key]).find((group) => group.grouping[key] === currentGroup.grouping[key]);
    if (!baselineGroup) {
      return { [key]: currentGroup.grouping[key], status: "insufficient_data", reason: "baseline_key_absent" };
    }
    const current = valueSelector(currentGroup.events);
    const baseline = valueSelector(baselineGroup.events);
    if (current === null || baseline === null) {
      return { [key]: currentGroup.grouping[key], status: "insufficient_data", reason: "metric_denominator_absent" };
    }
    return { [key]: currentGroup.grouping[key], status: "computed", current, baseline, delta: Number((current - baseline).toFixed(6)) };
  });
}

function regressions(currentEvents, baselineEvents) {
  const precisionValue = (events) => labelMetrics(events).precision.value;
  const latencyValue = (events) => latencyStats(events).p95_ms;
  const errorValue = (events) => {
    const rates = Object.values(errorRates(events)).filter((entry) => entry.status === "computed");
    if (rates.length === 0) return null;
    return Number(rates.reduce((total, entry) => total + entry.value, 0).toFixed(6));
  };
  return {
    baseline_release_ids: [...new Set(baselineEvents.map((event) => event.release_id))].sort(),
    model_precision: regressionRows(currentEvents, baselineEvents, "model_id", precisionValue),
    router_latency_p95_ms: regressionRows(currentEvents, baselineEvents, "router_version", latencyValue),
    provider_error_rate: regressionRows(currentEvents, baselineEvents, "provider_status", errorValue),
  };
}

function releaseDrift(currentEvents, baselineEvents) {
  const base = new Set(baselineEvents.map((event) => `${event.intent_type}|${event.decision_outcome}|${event.error_kind}`));
  return groupBy(currentEvents, ["release_id"]).map((group) => {
    const signatures = new Set(group.events.map((event) => `${event.intent_type}|${event.decision_outcome}|${event.error_kind}`));
    const newSignatures = [...signatures].filter((signature) => !base.has(signature)).sort();
    return {
      release_id: group.grouping.release_id,
      status: "computed",
      new_signature_count: newSignatures.length,
      baseline_signature_count: base.size,
      drift_score: Number((newSignatures.length / Math.max(1, signatures.size)).toFixed(6)),
      sanitized_signatures: newSignatures,
    };
  });
}

function falsePositiveClusters(events) {
  const clusters = new Map();
  for (const event of evaluated(events)) {
    if (event.expected_label === event.actual_label) continue;
    const grouping = {
      intent_type: event.intent_type,
      route: event.template_id,
      ...Object.fromEntries(falsePositiveKeys.slice(1).map((field) => [field, event[field]])),
    };
    const key = JSON.stringify(grouping);
    if (!clusters.has(key)) {
      clusters.set(key, {
        cluster_id: `fp-${clusters.size + 1}`,
        grouping,
        count: 0,
      });
    }
    clusters.get(key).count += 1;
  }
  return [...clusters.values()].sort((left, right) => right.count - left.count || left.cluster_id.localeCompare(right.cluster_id));
}

function readValidated(input, outDir, prefix) {
  const validation = validateInputFixture(input);
  if (validation.validation && !prefix) writeInputValidation(outDir, validation.validation);
  if (validation.status !== "PASS") return { status: "FAIL", errors: validation.errors, root: null };
  return { status: "PASS", errors: [], root: readJson(input) };
}

export function metrics(args) {
  const current = readValidated(args.input, args.outDir, "");
  if (current.status !== "PASS") return current;
  const baseline = readValidated(args.baseline, args.outDir, "baseline");
  if (baseline.status !== "PASS") return baseline;
  const currentEvents = current.root.events;
  const baselineEvents = baseline.root.events;
  writeJson(path.join(args.outDir, "metrics.json"), {
    schema_version: "phase6_cloud_eval_monitoring_metrics_v1",
    generated_at_utc: current.root.generated_at_utc,
    source_fixture_id: current.root.fixture_id,
    baseline_fixture_id: baseline.root.fixture_id,
    threshold_metadata: thresholds,
    dimensions: summarizeDimensions(currentEvents),
    metric_families: {
      precision_recall_by_intent_type: currentAndGrouped(currentEvents).by_intent_type,
      precision_recall_by_app_model_router_release: currentAndGrouped(currentEvents).by_app_model_router_release,
      approval_rejection_edit_rates: {
        overall: outcomeRates(currentEvents),
        by_app_model_router_release: groupBy(currentEvents, sliceKeys).map((group) => ({ ...group.grouping, ...outcomeRates(group.events) })),
      },
      latency_percentiles: {
        overall: latencyStats(currentEvents),
        by_app_model_router_release: groupBy(currentEvents, sliceKeys).map((group) => ({ ...group.grouping, ...latencyStats(group.events) })),
      },
      error_rates_by_error_kind: {
        overall: errorRates(currentEvents),
        by_app_model_router_release: groupBy(currentEvents, sliceKeys).map((group) => ({ ...group.grouping, error_rates: errorRates(group.events) })),
      },
      regressions_against_baseline_release: regressions(currentEvents, baselineEvents),
      drift_by_release: releaseDrift(currentEvents, baselineEvents),
      false_positive_clusters: falsePositiveClusters(currentEvents),
    },
    denominator_policy: {
      unavailable_denominators: "insufficient_data",
      deletion_suppressed_records: "audit_only_excluded_from_denominators",
    },
  });
  return { status: "PASS", errors: [], outDir: args.outDir };
}
