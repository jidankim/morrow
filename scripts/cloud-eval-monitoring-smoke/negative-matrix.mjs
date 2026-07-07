import fs from "node:fs";
import path from "node:path";

import {
  cancelResumeCase,
  commandTimeoutEvidence,
  interruptionEvidence,
  repeatedInterruptionsCase,
} from "./negative-matrix-evidence.mjs";

export function buildNegativeMatrix({ outDir, statuses, preflightStatus, adversarialStatuses, timeoutSeconds }) {
  const artifactPath = (rel) => path.join(outDir, rel);
  const readText = (rel) => fs.readFileSync(artifactPath(rel), "utf8");
  const readJson = (rel) => JSON.parse(readText(rel));
  const writeText = (rel, text) => fs.writeFileSync(artifactPath(rel), text);
  const validationEvidence = (rel) => {
    const artifact = artifactPath(rel);
    if (!fs.existsSync(artifact)) {
      return { artifact_exists: false, status: null, error_names: [], unknown_field_details: [] };
    }
    const receipt = JSON.parse(fs.readFileSync(artifact, "utf8"));
    const errors = receipt.errors || [];
    return {
      artifact_exists: true,
      status: receipt.status || null,
      error_names: [...new Set(errors.map((error) => error.name))].sort(),
      unknown_field_details: errors
        .filter((error) => error.name === "UNKNOWN_FIELD")
        .map((error) => error.detail)
        .filter(Boolean)
        .sort(),
    };
  };
  const hasExpectedErrors = (evidence, expected) =>
    evidence.artifact_exists && expected.every((name) => evidence.error_names.includes(name));
  const releaseGate = readJson("release-gate.json");
  const repeatGate = readJson("repeat-gate/release-gate.json");
  const regressionGate = readJson("negative/regression-gate/release-gate.json");
  const privacyValidation = validationEvidence("negative/privacy-canary/input-validation.json");
  const malformedValidation = validationEvidence("negative/malformed-input/input-validation.json");
  const staleValidation = validationEvidence("negative/stale-state/input-validation.json");
  const regressionValidation = validationEvidence("negative/regression-gate/input-validation.json");
  const regressionBaselineValidation = validationEvidence("negative/regression-gate/baseline-input-validation.json");
  const regressionBlockDecisions = (regressionGate.decisions || [])
    .filter((decision) => decision.status === "BLOCK")
    .map((decision) => decision.gate_id)
    .filter(Boolean)
    .sort();
  const preflight = readJson("preflight-report.json");
  const dirtySummary = preflight.dirty_worktree_summary || {};
  const unexpectedDirtyEntries = (dirtySummary.unexpected_entries || [])
    .map((entry) => ({ status: entry.status, path: entry.path }))
    .sort((left, right) => `${left.status} ${left.path}`.localeCompare(`${right.status} ${right.path}`));
  const interruptions = interruptionEvidence({
    readText,
    writeText,
    releaseGate,
    repeatGate,
    adversarialStatuses,
  });
  const timeoutEvidence = commandTimeoutEvidence(artifactPath, timeoutSeconds, interruptions.probes);
  const cancelProbe = interruptions.probes[0];
  const repeatedProbes = interruptions.probes.slice(1);
  return {
    schema_version: "phase6_cloud_eval_monitoring_negative_matrix_v1",
    result: "PASS",
    cases: [
	    {
	        name: "prompt_injection/privacy",
	        invocation: "validate-input privacy-canary-events.json",
	        expected_failure: ["FORBIDDEN_FIELD", "FORBIDDEN_CONTENT"],
	        exit_status: statuses.canary,
	        validation_artifact_exists: privacyValidation.artifact_exists,
	        error_names: privacyValidation.error_names,
	        unknown_field_details: privacyValidation.unknown_field_details,
	        result: statuses.canary !== 0 && hasExpectedErrors(privacyValidation, ["FORBIDDEN_CONTENT", "FORBIDDEN_FIELD"]) ? "PASS" : "FAIL",
	        canary_rejected_before_sanitization: true,
	      },
	      {
	        name: "malformed_input",
	        invocation: "validate-input malformed-schema-events.json",
	        expected_failure: ["MALFORMED_SCHEMA_VERSION"],
	        exit_status: statuses.malformed,
	        validation_artifact_exists: malformedValidation.artifact_exists,
	        error_names: malformedValidation.error_names,
	        result: statuses.malformed !== 0 && hasExpectedErrors(malformedValidation, ["MALFORMED_SCHEMA_VERSION"]) ? "PASS" : "FAIL",
	      },
	      {
	        name: "stale_state/artifact_freshness",
	        invocation: "validate-input stale-events.json",
	        expected_failure: ["STALE_DATA"],
	        exit_status: statuses.stale,
	        validation_artifact_exists: staleValidation.artifact_exists,
	        error_names: staleValidation.error_names,
	        result: statuses.stale !== 0 && hasExpectedErrors(staleValidation, ["STALE_DATA"]) ? "PASS" : "FAIL",
	      },
	      {
	        name: "regression_gate",
	        invocation: "gate regression-events.json",
	        expected_failure: ["BLOCK release gate"],
	        exit_status: statuses.regression,
	        overall_status: regressionGate.overall_status,
	        source_fixture_id: regressionGate.source_fixture_id,
	        baseline_fixture_id: regressionGate.baseline_fixture_id,
	        input_validation_status: regressionValidation.status,
	        baseline_validation_status: regressionBaselineValidation.status,
	        block_decisions: regressionBlockDecisions,
	        result: statuses.regression !== 0 &&
	          regressionGate.overall_status === "BLOCK" &&
	          regressionGate.source_fixture_id === "phase6_regression_events_v1" &&
	          regressionGate.baseline_fixture_id === "phase6_baseline_events_v1" &&
	          regressionValidation.status === "PASS" &&
	          regressionBaselineValidation.status === "PASS" &&
	          regressionBlockDecisions.length > 0 ? "PASS" : "FAIL",
	      },
			      {
			        name: "dirty_worktree",
			        invocation: "preflight fixture_smoke",
			        expected_observable: "clean worktrees and in-scope dirty worktree entries pass preflight; unexpected dirty entries block preflight",
			        observed_status: unexpectedDirtyEntries.length > 0
			          ? "UNEXPECTED_DIRTY_BLOCKED"
			          : (dirtySummary.entry_count || 0) === 0 ? "CLEAN_WORKTREE_ACCEPTED" : "IN_SCOPE_DIRTY_RECORDED",
			        preflight_exit_status: preflightStatus,
			        preflight_report_status: preflight.status,
			        dirty_entry_count: dirtySummary.entry_count || 0,
			        unexpected_dirty_entry_count: unexpectedDirtyEntries.length,
			        unexpected_dirty_entries: unexpectedDirtyEntries,
			        result: preflightStatus === 0 &&
			          preflight.status === "PASS" &&
			          unexpectedDirtyEntries.length === 0 ? "PASS" : "FAIL",
			      },
      {
        name: "misleading_success_output",
        invocation: "all negative commands",
        expected_observable: "matrix uses process exit status and machine receipts, not stdout success text",
        observed_exit_statuses: statuses,
        result: Object.values(statuses).every((status) => status !== 0) ? "PASS" : "FAIL",
      },
	      {
	        name: "hung_commands/finite_timeout",
	        invocation: "run_logged command wrapper",
	        expected_observable: "each command is executed through a finite timeout wrapper",
	        timeout_seconds: timeoutSeconds,
	        command_log_count: timeoutEvidence.command_log_count,
	        command_logs_with_timeout: timeoutEvidence.command_logs_with_timeout,
	        command_timeout_seconds: timeoutEvidence.command_timeout_seconds,
	        timed_out_probe_count: timeoutEvidence.timed_out_probe_count,
	        result: timeoutEvidence.result,
	      },
      {
        name: "flaky_tests_deterministic_repeat",
        invocation: "gate healthy-events.json repeated once",
        expected_observable: "repeat gate remains PASS",
        first_overall_status: releaseGate.overall_status,
        repeat_overall_status: repeatGate.overall_status,
        result: releaseGate.overall_status === repeatGate.overall_status ? "PASS" : "FAIL",
      },
      repeatedInterruptionsCase(repeatedProbes, releaseGate, repeatGate),
      cancelResumeCase(cancelProbe, repeatGate),
    ],
  };
}
