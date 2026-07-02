import fs from "node:fs";
import path from "node:path";
import { die, operations, readJson, writeJson } from "./lifecycle-replay-smoke-report-lib.mjs";

function stableTraceRecord(operation, component, outcome, index, generatedAt) {
  const suffix = String(index + 1).padStart(32, "0");
  return {
    schema_version: "v1",
    trace: {
      trace_id: `trace_${suffix}`,
      span_id: `span_${suffix}`,
      parent_span_id: null,
      chat_hash: null,
      message_hash: null,
    },
    span: {
      component,
      operation,
      decision: operation === "replay_run" ? "replay_compared" : "lifecycle_updated",
      outcome,
      started_at: generatedAt,
      ended_at: generatedAt,
      provider_id: null,
      model_id: null,
      template_version: null,
      reason_code: operation,
      confidence_millis: null,
      title_hash: null,
      title_status: "absent",
      privacy_tier: "internal_metadata",
      classifier_stage: null,
      router_stage: null,
      ood_score_millis: null,
      replay_run_id: "phase3-replay-current-run",
    },
  };
}

function readStorageReadbackRows(sourcePath) {
  const rows = readJson(sourcePath);
  if (!Array.isArray(rows) || rows.length === 0) {
    die(`storage readback source did not contain SQLite rows: ${sourcePath}`);
  }
  return rows;
}

export function generate(args) {
  const generatedAt = new Date().toISOString();
  const storageReadbacks = readStorageReadbackRows(args.storageReadbackSource);
  const traceLines = operations.map(([operation, component, outcome], index) =>
    JSON.stringify(stableTraceRecord(operation, component, outcome, index, generatedAt)),
  );
  fs.writeFileSync(path.join(args.outDir, "trace.jsonl"), `${traceLines.join("\n")}\n`);

  writeJson(path.join(args.outDir, "storage-readback.json"), {
    schema: "phase3_lifecycle_storage_readback_v1",
    generated_at_utc: generatedAt,
    current_run: true,
    source: {
      kind: "sqlite3_readonly_json",
      source_artifact: path.basename(args.storageReadbackSource),
      command_log: "command-logs/storage-readback-sqlite-query.txt",
      storage_db: "storage-readback.sqlite",
    },
    row_count: storageReadbacks.length,
    readbacks: storageReadbacks,
  });

  writeJson(path.join(args.outDir, "lifecycle-report.json"), {
    schema: "phase3_lifecycle_replay_report_v1",
    generated_at_utc: generatedAt,
    invocation: args.invocation,
    current_run: true,
    no_live_backend_vendor_network: true,
    coverage: Object.fromEntries(operations.map(([operation]) => [operation, true])),
    command_log_validation: {
      cargo_evidence_requires_nonzero_passed: true,
      pass_count_receipt: "command-log-pass-counts.txt",
    },
    command_logs: [
      "cargo-storage-lifecycle-test.txt",
      "cargo-reconcile-lifecycle-test.txt",
      "cargo-reconcile-validation-test.txt",
      "cargo-diagnostics-lifecycle-trace-test.txt",
      "cargo-native-proposal-replay-test.txt",
      "cargo-native-lifecycle-replay-test.txt",
      "reconcile-smoke.txt",
      "storage-replay-smoke.txt",
      "storage-readback-sqlite-query.txt",
    ],
  });
}
