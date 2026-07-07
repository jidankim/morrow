import { phase6SmokeCommand } from "./config.mjs"

export function parseJson(text, artifactName) {
  try {
    return JSON.parse(text)
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    throw new Error(`${artifactName} is not valid JSON: ${message}`)
  }
}

export function summaryFailures(summaryText, expectedSmokeCommand = phase6SmokeCommand) {
  const required = [
    "scenario: Phase 6 cloud eval monitoring smoke",
    `invocation: ${expectedSmokeCommand}`,
    "backend_observable: no live backend, provider network, EventKit, Messages, Calendar, Reminders, Phoenix, Langfuse, vendor backend, deployment, or cloud upload surface was used",
    "coverage: preflight",
    "coverage: input_validation",
    "coverage: aggregation",
    "coverage: metrics",
    "coverage: dashboard",
    "coverage: release_gate",
    "coverage: privacy_canary_rejection",
    "coverage: malformed_input_negative",
    "coverage: regression_negative",
    "coverage: stale_state_negative",
    "coverage: artifact_freshness",
    "result: PASS"
  ]
  return required
    .filter((phrase) => !summaryText.includes(phrase))
    .map((phrase) => `smoke summary missing required observable: ${phrase}`)
}

export function reportFailures(report) {
  const failures = []
  if (report.schema_version !== "phase6_cloud_eval_monitoring_dashboard_report_v1") {
    failures.push("dashboard report schema mismatch")
  }
  if (report.schema_validation?.status !== "PASS") {
    failures.push("dashboard report schema validation is not PASS")
  }
  for (const section of [
    "app_version",
    "model_version",
    "router_version",
    "release",
    "intent_type",
    "precision_recall",
    "approval_rejection_edit_rates",
    "latency_error_regressions",
    "drift",
    "false_positive_clusters",
    "gate_status"
  ]) {
    if (!(section in (report.sections ?? {}))) failures.push(`dashboard report missing section: ${section}`)
  }
  return failures
}

export function gateFailures(gate) {
  const failures = []
  if (gate.schema_version !== "phase6_cloud_eval_monitoring_release_gate_v1") {
    failures.push("release gate schema mismatch")
  }
  if (gate.overall_status !== "PASS") failures.push("release gate is not PASS")
  if (gate.deployment_action !== "none") failures.push("release gate must not request deployment action")
  return failures
}

export function negativeMatrixFailures(matrix) {
  const failures = []
  if (matrix.schema_version !== "phase6_cloud_eval_monitoring_negative_matrix_v1") {
    failures.push("negative matrix schema mismatch")
  }
  if (matrix.result !== "PASS") failures.push("negative matrix is not PASS")
  const requiredCases = [
    "prompt_injection/privacy",
    "malformed_input",
    "stale_state/artifact_freshness",
    "regression_gate",
    "dirty_worktree",
    "misleading_success_output"
  ]
  for (const requiredCase of requiredCases) {
    if (!matrix.cases?.some((entry) => entry.name === requiredCase && entry.result === "PASS")) {
      failures.push(`negative matrix missing passing case: ${requiredCase}`)
    }
  }
  return failures
}

export function cleanupFailures(cleanupText) {
  const required = [
    "scenario: phase 6 cloud eval monitoring cleanup",
    "observable: no background services or provider/native/network processes were started by the smoke runner",
    "result: PASS"
  ]
  return required
    .filter((phrase) => !cleanupText.includes(phrase))
    .map((phrase) => `cleanup receipt missing required observable: ${phrase}`)
}

export function patternFailures(text, patterns, prefix) {
  return patterns.filter(({ pattern }) => pattern.test(text)).map(({ name }) => `${prefix}: ${name}`)
}
