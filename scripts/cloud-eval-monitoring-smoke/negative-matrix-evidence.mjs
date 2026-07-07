import fs from "node:fs";
import path from "node:path";

export function commandTimeoutEvidence(artifactPath, timeoutSeconds, probes) {
  const commandLogDir = artifactPath("command-logs");
  const expectedLogs = [
    "adversarial-cancel-resume.txt",
    "adversarial-repeated-interruptions-1.txt",
    "adversarial-repeated-interruptions-2.txt",
    "dashboard.txt",
    "deterministic-repeat-gate.txt",
    "gate.txt",
    "generate.txt",
    "metrics.txt",
    "negative-malformed-input.txt",
    "negative-privacy-canary.txt",
    "negative-regression-gate.txt",
    "negative-stale-state.txt",
    "preflight.txt",
    "validate-input.txt",
  ];
  const commandLogs = expectedLogs.map((name) => {
    const logPath = path.join(commandLogDir, name);
    const exists = fs.existsSync(logPath);
    const text = exists ? fs.readFileSync(logPath, "utf8") : "";
    const timeoutMatch = /^timeout_seconds:\s*(\d+)$/m.exec(text);
    const recordedTimeoutSeconds = timeoutMatch ? Number(timeoutMatch[1]) : null;
    return {
      path: `command-logs/${name}`,
      exists,
      timeout_seconds: recordedTimeoutSeconds,
      has_timeout_seconds: Number.isFinite(recordedTimeoutSeconds) && recordedTimeoutSeconds > 0,
      has_exit_status: /(^|\n)exit_status:\s*\d+/.test(text),
    };
  });
  const commandLogsWithTimeout = commandLogs
    .filter((log) => log.exists && log.has_timeout_seconds && log.has_exit_status)
    .map((log) => log.path)
    .sort();
  const timedOutProbeCount = probes.filter((probe) => probe.timeout_observed).length;
  return {
    command_log_count: commandLogs.filter((log) => log.exists).length,
    command_logs_with_timeout: commandLogsWithTimeout,
    command_timeout_seconds: commandLogs
      .filter((log) => log.exists)
      .map((log) => ({ path: log.path, timeout_seconds: log.timeout_seconds }))
      .sort((left, right) => left.path.localeCompare(right.path)),
    timed_out_probe_count: timedOutProbeCount,
    result: Number.isFinite(timeoutSeconds) &&
      timeoutSeconds > 0 &&
      commandLogsWithTimeout.length === expectedLogs.length &&
      timedOutProbeCount === probes.length ? "PASS" : "FAIL",
  };
}

function readReceipt(readText, rel) {
  const values = {};
  for (const line of readText(rel).split(/\r?\n/)) {
    const match = /^([^:]+):\s*(.*)$/.exec(line);
    if (match) values[match[1]] = match[2];
  }
  return values;
}

function interruptionProbe(readText, rel, logRel, status) {
  const receipt = readReceipt(readText, rel);
  const log = readText(logRel);
  return {
    receipt: rel,
    command_log: logRel,
    label: receipt.label,
    invocation: `node scripts/cloud-eval-monitoring-smoke/run-with-timeout.mjs 1 bash scripts/cloud-eval-monitoring-smoke/interruption-probe-child.sh ${rel} ${receipt.label} <unique-token>`,
    exit_status: status,
    timeout_observed: log.includes("timeout_after_seconds: 1"),
    cleanup_receipt_written: receipt.cleanup_receipt_written === "yes",
    observed_signal: receipt.observed_signal,
    child_pid: Number(receipt.child_pid),
    process_absence: log.includes("process_absence: PASS") ? "PASS" : "FAIL",
  };
}

export function interruptionEvidence({ readText, writeText, releaseGate, repeatGate, adversarialStatuses }) {
  const probes = [
    interruptionProbe(readText, "adversarial/cancel-resume.receipt.txt", "command-logs/adversarial-cancel-resume.txt", adversarialStatuses.cancelResume),
    interruptionProbe(readText, "adversarial/repeated-interruptions-1.receipt.txt", "command-logs/adversarial-repeated-interruptions-1.txt", adversarialStatuses.repeatedInterruptionsFirst),
    interruptionProbe(readText, "adversarial/repeated-interruptions-2.receipt.txt", "command-logs/adversarial-repeated-interruptions-2.txt", adversarialStatuses.repeatedInterruptionsSecond),
  ];
  const complete = (probe) =>
    probe.exit_status !== 0 &&
    probe.timeout_observed &&
    probe.cleanup_receipt_written &&
    probe.observed_signal === "TERM" &&
    probe.process_absence === "PASS";
  const evidence = {
    schema_version: "phase6_cloud_eval_monitoring_interruption_evidence_v1",
    probes,
    post_interruption_gate: {
      first_overall_status: releaseGate.overall_status,
      repeat_overall_status: repeatGate.overall_status,
      result: releaseGate.overall_status === "PASS" && repeatGate.overall_status === "PASS" ? "PASS" : "FAIL",
    },
  };
  evidence.result = probes.every(complete) && evidence.post_interruption_gate.result === "PASS" ? "PASS" : "FAIL";
  writeText("adversarial-interruptions.json", `${JSON.stringify(evidence, null, 2)}\n`);
  return evidence;
}

export function repeatedInterruptionsCase(probes, releaseGate, repeatGate) {
  return {
    name: "repeated_interruptions",
    invocation: "two timeout-wrapped interruption child commands followed by gate healthy-events.json repeated once",
    expected_observable: "both interrupted children exit non-zero, write TERM cleanup receipts, leave no matching process, and the healthy repeat gate remains PASS",
    exit_statuses: probes.map((probe) => probe.exit_status),
    cleanup_receipts: probes.map((probe) => probe.receipt),
    timeout_observed: probes.every((probe) => probe.timeout_observed),
    process_absence: probes.every((probe) => probe.process_absence === "PASS") ? "PASS" : "FAIL",
    first_overall_status: releaseGate.overall_status,
    repeat_overall_status: repeatGate.overall_status,
    result: probes.every((probe) => probe.exit_status !== 0) &&
      probes.every((probe) => probe.cleanup_receipt_written && probe.observed_signal === "TERM") &&
      probes.every((probe) => probe.process_absence === "PASS") &&
      repeatGate.overall_status === "PASS" ? "PASS" : "FAIL",
  };
}

export function cancelResumeCase(probe, repeatGate) {
  return {
    name: "cancel_resume",
    invocation: probe.invocation,
    expected_observable: "timeout wrapper terminates a trapped child, the child writes a cleanup receipt, the wrapper exits non-zero, no matching child process remains, and the smoke resumes to a healthy gate",
    exit_status: probe.exit_status,
    cleanup_receipt: probe.receipt,
    observed_signal: probe.observed_signal,
    timeout_observed: probe.timeout_observed,
    process_absence: probe.process_absence,
    post_cancel_resume_repeat_overall_status: repeatGate.overall_status,
    result: probe.exit_status !== 0 &&
      probe.cleanup_receipt_written &&
      probe.observed_signal === "TERM" &&
      probe.process_absence === "PASS" &&
      repeatGate.overall_status === "PASS" ? "PASS" : "FAIL",
  };
}
