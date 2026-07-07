import fs from "node:fs";
import path from "node:path";
import { gateThresholds } from "./cloud-eval-monitoring-gates.mjs";
import { metrics } from "./cloud-eval-monitoring-metrics.mjs";
import { readJson, writeJson } from "./cloud-eval-monitoring-report-lib.mjs";
import { scanForbiddenContent } from "./cloud-eval-monitoring-report/privacy.mjs";

const reportSchema = "phase6_cloud_eval_monitoring_dashboard_report_v1";

function fmtMetric(metric) {
  if (!metric || metric.status !== "computed") return "insufficient_data";
  return `${metric.value} (${metric.numerator}/${metric.denominator})`;
}

function joinValues(values) {
  return values.length === 0 ? "none" : values.join(", ");
}

function maxStatus(statuses) {
  if (statuses.includes("BLOCK")) return "BLOCK";
  if (statuses.includes("WARN")) return "WARN";
  return "PASS";
}

function gateStatus(metricReport) {
  const thresholds = gateThresholds;
  const families = metricReport.metric_families;
  const checks = [];
  for (const row of families.precision_recall_by_intent_type) {
    for (const name of ["precision", "recall"]) {
      const metric = row[name];
      if (metric.status !== "computed") {
        checks.push({ status: "WARN", check: `${name}_by_intent_type`, detail: `${row.intent_type}:insufficient_data` });
      } else if (metric.value < thresholds.hard_limits[`${name}_min`]) {
        checks.push({ status: "BLOCK", check: `${name}_by_intent_type`, detail: `${row.intent_type}:${metric.value}` });
      } else if (metric.value < thresholds.soft_limits[`${name}_warn_below`]) {
        checks.push({ status: "WARN", check: `${name}_by_intent_type`, detail: `${row.intent_type}:${metric.value}` });
      }
    }
  }
  for (const row of families.false_positive_clusters) {
    if (row.count > thresholds.hard_limits.false_positive_cluster_max_count) {
      checks.push({ status: "BLOCK", check: "false_positive_cluster_size", detail: `${row.cluster_id}:${row.count}` });
    } else if (row.count >= thresholds.soft_limits.false_positive_cluster_warn_count) {
      checks.push({ status: "WARN", check: "false_positive_cluster_size", detail: `${row.cluster_id}:${row.count}` });
    }
  }
  for (const row of families.regressions_against_baseline_release.router_latency_p95_ms) {
    if (row.status === "computed" && row.baseline > 0) {
      const ratio = row.current / row.baseline;
      if (ratio > thresholds.hard_limits.latency_p95_regression_max_ratio || row.delta > thresholds.hard_limits.latency_p95_regression_max_delta_ms) {
        checks.push({ status: "BLOCK", check: "latency_p95_regression", detail: `${row.router_version}:${row.current}/${row.baseline}` });
      } else if (ratio > thresholds.soft_limits.latency_p95_regression_warn_ratio) {
        checks.push({ status: "WARN", check: "latency_p95_regression", detail: `${row.router_version}:${row.current}/${row.baseline}` });
      }
    }
  }
  return {
    status: maxStatus(checks.map((check) => check.status)),
    basis: checks.length === 0 ? [{ status: "PASS", check: "dashboard_metric_projection", detail: "all dashboard checks pass" }] : checks,
  };
}

function buildReport(metricReport) {
  const families = metricReport.metric_families;
  return {
    schema_version: reportSchema,
    schema_validation: { status: "PASS", validator: "cloud-eval-monitoring-dashboard.mjs" },
    generated_at_utc: metricReport.generated_at_utc,
    source_fixture_id: metricReport.source_fixture_id,
    baseline_fixture_id: metricReport.baseline_fixture_id,
    dimensions: metricReport.dimensions,
    sections: {
      app_version: metricReport.dimensions.dimension_sets.app_versions,
      model_version: metricReport.dimensions.dimension_sets.model_versions,
      router_version: metricReport.dimensions.dimension_sets.router_versions,
      release: {
        release_ids: metricReport.dimensions.dimension_sets.release_ids,
        release_channels: metricReport.dimensions.dimension_sets.release_channels,
        baseline_release_ids: families.regressions_against_baseline_release.baseline_release_ids,
      },
      intent_type: metricReport.dimensions.dimension_sets.intent_types,
      precision_recall: {
        by_intent_type: families.precision_recall_by_intent_type,
        by_app_model_router_release: families.precision_recall_by_app_model_router_release,
      },
      approval_rejection_edit_rates: families.approval_rejection_edit_rates,
      latency_error_regressions: {
        latency_percentiles: families.latency_percentiles,
        error_rates_by_error_kind: families.error_rates_by_error_kind,
        regressions_against_baseline_release: families.regressions_against_baseline_release,
      },
      drift: families.drift_by_release,
      false_positive_clusters: families.false_positive_clusters,
      gate_status: gateStatus(metricReport),
    },
  };
}

function validateReport(report) {
  const errors = [];
  const required = ["app_version", "model_version", "router_version", "release", "intent_type", "precision_recall",
    "approval_rejection_edit_rates", "latency_error_regressions", "drift", "false_positive_clusters", "gate_status"];
  if (report.schema_version !== reportSchema) errors.push({ name: "DASHBOARD_SCHEMA_INVALID", detail: "schema_version" });
  for (const key of required) {
    if (!Object.hasOwn(report.sections, key)) errors.push({ name: "DASHBOARD_SCHEMA_INVALID", detail: `sections.${key}` });
  }
  scanForbiddenContent(JSON.stringify(report), errors);
  return errors;
}

function renderMarkdown(report) {
  const sections = report.sections;
  const lines = [
    "# Cloud Eval Monitoring Dashboard",
    "",
    `Generated: ${report.generated_at_utc}`,
    `Source fixture: ${report.source_fixture_id}`,
    `Baseline fixture: ${report.baseline_fixture_id}`,
    "",
    "## App Version",
    joinValues(sections.app_version),
    "",
    "## Model Version",
    joinValues(sections.model_version),
    "",
    "## Router Version",
    joinValues(sections.router_version),
    "",
    "## Release",
    `Current: ${joinValues(sections.release.release_ids)}`,
    `Channels: ${joinValues(sections.release.release_channels)}`,
    `Baseline: ${joinValues(sections.release.baseline_release_ids)}`,
    "",
    "## Intent Type",
    joinValues(sections.intent_type),
    "",
    "## Precision/Recall",
    "| Intent type | Precision | Recall |",
    "| --- | --- | --- |",
    ...sections.precision_recall.by_intent_type.map((row) => `| ${row.intent_type} | ${fmtMetric(row.precision)} | ${fmtMetric(row.recall)} |`),
    "",
    "## Approval/Rejection/Edit Rates",
    `Approval: ${fmtMetric(sections.approval_rejection_edit_rates.overall.approval_rate)}`,
    `Rejection: ${fmtMetric(sections.approval_rejection_edit_rates.overall.rejection_rate)}`,
    `Edit: ${fmtMetric(sections.approval_rejection_edit_rates.overall.edit_rate)}`,
    "",
    "## Latency/Error Regressions",
    `Latency p95 overall: ${sections.latency_error_regressions.latency_percentiles.overall.p95_ms ?? "insufficient_data"} ms`,
    ...sections.latency_error_regressions.regressions_against_baseline_release.router_latency_p95_ms.map((row) => `Latency ${row.router_version}: ${row.status === "computed" ? `${row.current} ms vs ${row.baseline} ms` : row.reason}`),
    "",
    "## Drift",
    ...sections.drift.map((row) => `${row.release_id}: score ${row.drift_score}, new signatures ${row.new_signature_count}`),
    "",
    "## False-Positive Clusters",
    ...(sections.false_positive_clusters.length === 0
      ? ["none"]
      : sections.false_positive_clusters.map((row) => `${row.cluster_id}: ${row.count} ${row.grouping.intent_type}/${row.grouping.route_reason_code}`)),
    "",
    "## Gate Status",
    sections.gate_status.status,
    ...sections.gate_status.basis.map((row) => `- ${row.status}: ${row.check} ${row.detail}`),
    "",
  ];
  return `${lines.join("\n")}\n`;
}

function renderSummary(report) {
  const gate = report.sections.gate_status;
  return [
    `schema_version: ${report.schema_version}`,
    `status: ${gate.status}`,
    `source_fixture_id: ${report.source_fixture_id}`,
    `baseline_fixture_id: ${report.baseline_fixture_id}`,
    `app_versions: ${joinValues(report.sections.app_version)}`,
    `model_versions: ${joinValues(report.sections.model_version)}`,
    `router_versions: ${joinValues(report.sections.router_version)}`,
    `release_ids: ${joinValues(report.sections.release.release_ids)}`,
    `intent_types: ${joinValues(report.sections.intent_type)}`,
    `false_positive_clusters: ${report.sections.false_positive_clusters.length}`,
    "",
  ].join("\n");
}

export function dashboard(args) {
  const metricResult = metrics(args);
  if (metricResult.status !== "PASS") return metricResult;
  const metricReport = readJson(path.join(args.outDir, "metrics.json"));
  const report = buildReport(metricReport);
  const errors = validateReport(report);
  const markdown = renderMarkdown(report);
  const summary = renderSummary(report);
  scanForbiddenContent(markdown, errors);
  scanForbiddenContent(summary, errors);
  if (errors.length > 0) return { status: "FAIL", errors };
  writeJson(path.join(args.outDir, "cloud-eval-monitoring-report.json"), report);
  fs.writeFileSync(path.join(args.outDir, "cloud-eval-monitoring-dashboard.md"), markdown);
  fs.writeFileSync(path.join(args.outDir, "dashboard-summary.txt"), summary);
  return { status: "PASS", errors: [], outDir: args.outDir };
}
