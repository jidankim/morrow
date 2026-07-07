import fs from "node:fs";
import path from "node:path";
import { metrics } from "./cloud-eval-monitoring-metrics.mjs";
import { readJson, validateInputFixture, writeInputValidation, writeJson } from "./cloud-eval-monitoring-report-lib.mjs";

export const gateThresholds = {
  schema_version: "phase6_cloud_eval_monitoring_gate_thresholds_v1",
  thresholds_version: "2026-07-07.final-review.fix1.v1",
  required_slices: {
    min_app_versions: 2,
    min_model_versions: 2,
    min_router_versions: 2,
    min_release_ids: 2,
    intent_types: ["calendar_event", "task_reminder", "quiet_low_confidence"],
  },
  hard_limits: {
    precision_min: 0.9, recall_min: 0.9, false_positive_cluster_max_count: 3,
    latency_p95_regression_max_ratio: 20, latency_p95_regression_max_delta_ms: 5000, error_rate_max: 0.6,
  },
  soft_limits: {
    precision_warn_below: 0.9, recall_warn_below: 0.9, false_positive_cluster_warn_count: 2,
    latency_p95_regression_warn_ratio: 20, drift_score_warn_above: 0.75, minimum_evaluable_labels: 1,
  },
};

function addDecision(decisions, status, gateId, message, observed = {}, threshold = {}) {
  decisions.push({ gate_id: gateId, status, message, observed, threshold });
}

function overallStatus(decisions) {
  if (decisions.some((item) => item.status === "BLOCK")) return "BLOCK";
  if (decisions.some((item) => item.status === "WARN")) return "WARN";
  return "PASS";
}

function classifyValidationError(name) {
  if (["FORBIDDEN_FIELD", "FORBIDDEN_CONTENT", "SANITIZED_TOKEN_EXPECTED"].includes(name)) return "privacy";
  if (name === "STALE_DATA") return "stale";
  return "schema";
}

function addValidationDecisions(decisions, result, source) {
  if (result.status === "PASS") return false;
  const grouped = new Map();
  for (const error of result.errors) {
    const kind = classifyValidationError(error.name);
    const key = `${source}_${kind}`;
    const value = grouped.get(key) || { names: new Set(), count: 0 };
    value.names.add(error.name);
    value.count += 1;
    grouped.set(key, value);
  }
  for (const [key, value] of grouped) {
    addDecision(decisions, "BLOCK", `${key}_gate`, `${key.replaceAll("_", " ")} validation failed`, {
      error_names: [...value.names].sort(),
      error_count: value.count,
    });
  }
  return true;
}

function checkRequiredSlices(decisions, dimensions) {
  if (dimensions.evaluable_count < gateThresholds.soft_limits.minimum_evaluable_labels) {
    addDecision(decisions, "WARN", "insufficient_evaluable_labels", "no evaluable labels are available for hard slice gates", {
      evaluable_count: dimensions.evaluable_count,
    }, { minimum_evaluable_labels: gateThresholds.soft_limits.minimum_evaluable_labels });
    return;
  }
  const sets = dimensions.dimension_sets;
  for (const [name, minimum] of [
    ["app_versions", gateThresholds.required_slices.min_app_versions],
    ["model_versions", gateThresholds.required_slices.min_model_versions],
    ["router_versions", gateThresholds.required_slices.min_router_versions],
    ["release_ids", gateThresholds.required_slices.min_release_ids],
  ]) {
    if ((sets[name] || []).length < minimum) {
      addDecision(decisions, "BLOCK", `missing_required_${name}`, `missing required ${name} slices`, { values: sets[name] || [] }, { minimum });
    }
  }
  const missing = gateThresholds.required_slices.intent_types.filter((intent) => !sets.intent_types.includes(intent));
  if (missing.length > 0) {
    addDecision(decisions, "BLOCK", "missing_required_intent_types", "missing required intent slices", { missing_intent_types: missing }, {
      required_intent_types: gateThresholds.required_slices.intent_types,
    });
  }
}

function checkRatioMetric(decisions, gateId, row, metricName) {
  const metric = row[metricName];
  if (!metric || metric.status === "insufficient_data") {
    addDecision(decisions, "WARN", `${gateId}_${metricName}_insufficient`, `${metricName} has insufficient data`, { metric: metricName });
    return;
  }
  const hardMinimum = gateThresholds.hard_limits[`${metricName}_min`];
  const warnBelow = gateThresholds.soft_limits[`${metricName}_warn_below`];
  if (metric.value < hardMinimum) {
    addDecision(decisions, "BLOCK", `${gateId}_${metricName}_hard_min`, `${metricName} is below the hard minimum`, {
      value: metric.value,
      numerator: metric.numerator,
      denominator: metric.denominator,
    }, { minimum: hardMinimum });
  } else if (metric.value < warnBelow) {
    addDecision(decisions, "WARN", `${gateId}_${metricName}_soft_min`, `${metricName} is below the warning threshold`, { value: metric.value }, {
      warning_below: warnBelow,
    });
  }
}

function checkQualityMetrics(decisions, families) {
  for (const [familyName, rows] of [
    ["intent", families.precision_recall_by_intent_type], ["release_slice", families.precision_recall_by_app_model_router_release],
  ]) {
    rows.forEach((row, index) => {
      checkRatioMetric(decisions, `${familyName}_${index}`, row, "precision");
      checkRatioMetric(decisions, `${familyName}_${index}`, row, "recall");
    });
  }
}

function checkFalsePositiveClusters(decisions, clusters) {
  for (const cluster of clusters) {
    if (cluster.count > gateThresholds.hard_limits.false_positive_cluster_max_count) {
      addDecision(decisions, "BLOCK", "false_positive_cluster_hard_limit", "false-positive cluster exceeds hard limit", {
        cluster_id: cluster.cluster_id,
        count: cluster.count,
        grouping: cluster.grouping,
      }, { maximum: gateThresholds.hard_limits.false_positive_cluster_max_count });
    } else if (cluster.count >= gateThresholds.soft_limits.false_positive_cluster_warn_count) {
      addDecision(decisions, "WARN", "false_positive_cluster_uncertain", "false-positive cluster needs review", {
        cluster_id: cluster.cluster_id, count: cluster.count,
      }, { warning_count: gateThresholds.soft_limits.false_positive_cluster_warn_count });
    }
  }
}

function checkLatencyRegressions(decisions, rows) {
  for (const row of rows) {
    if (row.status !== "computed") {
      addDecision(decisions, "WARN", "latency_regression_insufficient", "latency regression has insufficient data", row);
      continue;
    }
    const ratio = row.baseline === 0 ? null : Number((row.current / row.baseline).toFixed(6));
    const observed = { router_version: row.router_version, current: row.current, baseline: row.baseline, delta: row.delta, ratio };
    if (ratio > gateThresholds.hard_limits.latency_p95_regression_max_ratio || row.delta > gateThresholds.hard_limits.latency_p95_regression_max_delta_ms) {
      addDecision(decisions, "BLOCK", "latency_regression_hard_limit", "latency p95 regression exceeds hard limit", observed, gateThresholds.hard_limits);
    } else if (ratio > gateThresholds.soft_limits.latency_p95_regression_warn_ratio) {
      addDecision(decisions, "WARN", "latency_regression_soft_limit", "latency p95 regression exceeds warning threshold", observed, gateThresholds.soft_limits);
    }
  }
}

function checkErrorRate(decisions, errorRates) {
  const entries = Object.values(errorRates).filter((entry) => entry.status === "computed");
  const value = Number(entries.reduce((total, entry) => total + entry.value, 0).toFixed(6));
  if (entries.length === 0) {
    addDecision(decisions, "WARN", "error_rate_insufficient", "error-rate regression has insufficient data");
  } else if (value > gateThresholds.hard_limits.error_rate_max) {
    addDecision(decisions, "BLOCK", "error_rate_hard_limit", "overall error rate exceeds hard limit", { value }, { maximum: gateThresholds.hard_limits.error_rate_max });
  }
}

function checkDrift(decisions, rows) {
  for (const row of rows) {
    if (row.drift_score > gateThresholds.soft_limits.drift_score_warn_above) {
      addDecision(decisions, "WARN", "release_drift_soft_threshold", "release drift exceeds warning threshold", {
        release_id: row.release_id, drift_score: row.drift_score, new_signature_count: row.new_signature_count,
      }, { warning_above: gateThresholds.soft_limits.drift_score_warn_above });
    }
  }
}

function alertsFor(overall, decisions) {
  const active = decisions.filter((item) => item.status !== "PASS");
  if (active.length === 0) return [{ alert_id: "release_gate_pass", status: "PASS", severity: "none", message: "all release gates satisfied" }];
  return active.map((item) => ({
    alert_id: item.gate_id,
    status: item.status,
    severity: item.status === "BLOCK" ? "release_blocking" : "review_required",
    message: item.message,
    observed: item.observed,
    threshold: item.threshold,
    overall_status: overall,
  }));
}

function renderMarkdown(report) {
  const lines = [
    "# Phase 6 Release Gate",
    "",
    `overall_status: ${report.overall_status}`,
    `source_fixture_id: ${report.source_fixture_id}`,
    `baseline_fixture_id: ${report.baseline_fixture_id}`,
    "",
    "## Decisions",
  ];
  report.decisions.forEach((item) => lines.push(`- ${item.status} ${item.gate_id}: ${item.message}`));
  lines.push("", "## Alerts");
  report.alerts.forEach((item) => lines.push(`- ${item.status} ${item.alert_id}: ${item.message}`));
  lines.push("", "No release promotion or deployment tooling was invoked.");
  return `${lines.join("\n")}\n`;
}

function writeGateArtifacts(args, metricsReport, decisions) {
  if (decisions.length === 0) addDecision(decisions, "PASS", "all_release_gates_satisfied", "all release gates satisfied");
  const overall = overallStatus(decisions);
  const alerts = alertsFor(overall, decisions);
  const report = {
    schema_version: "phase6_cloud_eval_monitoring_release_gate_v1",
    generated_at_utc: metricsReport?.generated_at_utc || new Date(0).toISOString(),
    source_fixture_id: metricsReport?.source_fixture_id || path.basename(args.input),
    baseline_fixture_id: metricsReport?.baseline_fixture_id || path.basename(args.baseline),
    overall_status: overall,
    threshold_metadata: gateThresholds,
    live_cloud_prerequisites: args.requireCloudPrerequisites ? "BLOCKED" : "not_required_for_fixture_gate",
    decisions,
    alerts,
    deployment_action: "none",
  };
  writeJson(path.join(args.outDir, "release-gate.json"), report);
  writeJson(path.join(args.outDir, "alerts.json"), { schema_version: "phase6_cloud_eval_monitoring_alerts_v1", overall_status: overall, alerts });
  fs.writeFileSync(path.join(args.outDir, "release-gate.md"), renderMarkdown(report));
  return report;
}

export function gate(args) {
  const decisions = [];
  if (args.requireCloudPrerequisites) {
    addDecision(decisions, "BLOCK", "live_cloud_prerequisites_blocked", "live cloud input requires collector prerequisite receipts");
  }
  const current = validateInputFixture(args.input);
  if (current.validation) writeInputValidation(args.outDir, current.validation);
  const baseline = validateInputFixture(args.baseline);
  if (baseline.validation) writeJson(path.join(args.outDir, "baseline-input-validation.json"), baseline.validation);
  const inputBlocked = addValidationDecisions(decisions, current, "input");
  const baselineBlocked = addValidationDecisions(decisions, baseline, "baseline");
  const blocked = inputBlocked || baselineBlocked;
  if (!blocked) {
    const metricResult = metrics(args);
    if (metricResult.status !== "PASS") {
      addDecision(decisions, "BLOCK", "metrics_generation_failed", "metrics generation failed", {
        error_names: metricResult.errors.map((error) => error.name),
      });
    } else {
      const report = readJson(path.join(args.outDir, "metrics.json"));
      checkRequiredSlices(decisions, report.dimensions);
      checkQualityMetrics(decisions, report.metric_families);
      checkFalsePositiveClusters(decisions, report.metric_families.false_positive_clusters);
      checkLatencyRegressions(decisions, report.metric_families.regressions_against_baseline_release.router_latency_p95_ms);
      checkErrorRate(decisions, report.metric_families.error_rates_by_error_kind.overall);
      checkDrift(decisions, report.metric_families.drift_by_release);
      return writeGateArtifacts(args, report, decisions);
    }
  }
  return writeGateArtifacts(args, null, decisions);
}
