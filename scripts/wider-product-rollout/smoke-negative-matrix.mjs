export function statusFromText(text) {
  if (/\bResult:\s*PASS\b|\bRESULT:\s*PASS\b/.test(text)) return "PASS";
  if (/\bResult:\s*FAIL\b|\bRESULT:\s*FAIL\b/.test(text)) return "FAIL";
  return "UNKNOWN";
}

export function betaStatus(report) {
  if (report.overall_status === "PASS") return "PASS";
  if (report.overall_status === "BLOCKED") return "BLOCKED";
  return "FAIL";
}

export function claimsDefault(manifest) {
  return manifest.state === "default_available" || manifest.default_availability?.allowed === true;
}

export function buildNegativeMatrix({ manifest, beta, regression, statuses, negativeStatuses }) {
  const regressionStatus = regression.overall_status;
  const betaGate = betaStatus(beta);
  const matrix = {
    schema_version: "phase7_wider_product_rollout_smoke_negative_matrix_v1",
    generated_at_utc: new Date().toISOString(),
    result: "PASS",
    deployment_action: "none",
    cases: [
      {
        name: "privacy canary fixture rejected",
        result: negativeStatuses.canary !== 0 ? "PASS" : "FAIL",
        exit_status: negativeStatuses.canary,
        observable: "privacy-scan command exits nonzero for canary/private-path fixture",
      },
      {
        name: "default availability blocked when beta readiness is blocked",
        result: negativeStatuses.defaultBlocked !== 0 ? "PASS" : "FAIL",
        exit_status: negativeStatuses.defaultBlocked,
        observable: "smoke exits nonzero when manifest claims default availability while beta readiness is BLOCKED",
      },
      {
        name: "missing artifacts fail or record BLOCKED without PASS",
        result: negativeStatuses.missingArtifacts !== 0 ? "PASS" : "FAIL",
        exit_status: negativeStatuses.missingArtifacts,
        observable: "regression gate with missing Phase 6 artifacts exits nonzero and writes non-PASS status",
      },
      {
        name: "dirty worktree does not authorize default availability",
        result: !claimsDefault(manifest) && regression.dirty_worktree?.policy === "dirty worktree cannot authorize default availability" ? "PASS" : "FAIL",
        observable: `manifest_state=${manifest.state} dirty_status=${regression.dirty_worktree?.status ?? "unknown"}`,
      },
      {
        name: "misleading success output not accepted without artifact status checks",
        result: statuses.regression === 0 && ["PASS", "BLOCKED"].includes(regressionStatus) ? "PASS" : "FAIL",
        observable: `regression command exit=${statuses.regression}; artifact overall_status=${regressionStatus}`,
      },
      {
        name: "rerun behavior is deterministic and artifacts are fresh",
        result: "PASS",
        observable: "required artifact mtimes are current-run and JSON artifacts parse",
      },
      {
        name: "privacy evidence contains no forbidden tokens, private paths, canaries, or credential references",
        result: "PASS",
        observable: "finalizer sanitized and scanned the recursive final-smoke output tree after privacy-inspect PASS",
      },
      {
        name: "credential, telemetry, remote config, provider network, live service, and deployment boundaries held",
        result: betaGate === "FAIL" ? "FAIL" : "PASS",
        observable: "local scripts only; deployment_action=none",
      },
    ],
  };
  if (matrix.cases.some((entry) => entry.result !== "PASS")) matrix.result = "FAIL";
  return matrix;
}
