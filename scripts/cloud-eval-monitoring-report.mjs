#!/usr/bin/env node
import { validateInputFixture, writeInputValidation, die } from "./cloud-eval-monitoring-report-lib.mjs";
import { generate } from "./cloud-eval-monitoring-report-generate.mjs";
import { metrics } from "./cloud-eval-monitoring-metrics.mjs";
import { dashboard } from "./cloud-eval-monitoring-dashboard.mjs";
import { gate } from "./cloud-eval-monitoring-gates.mjs";

function parseArgs(argv) {
  const args = { mode: argv[2], input: "", baseline: "", outDir: "", requireCloudPrerequisites: false };
  for (let index = 3; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--input") {
      args.input = argv[index + 1] || "";
      index += 1;
    } else if (arg === "--baseline") {
      args.baseline = argv[index + 1] || "";
      index += 1;
    } else if (arg === "--out-dir") {
      args.outDir = argv[index + 1] || "";
      index += 1;
    } else if (arg === "--require-cloud-prerequisites") {
      args.requireCloudPrerequisites = true;
    } else {
      die(`unknown argument: ${arg}`);
    }
  }
  if (!["validate-input", "generate", "metrics", "dashboard", "gate"].includes(args.mode)) {
    die("usage: cloud-eval-monitoring-report.mjs <validate-input|generate|metrics|dashboard|gate> --input <path> [--baseline <path>] --out-dir <path>");
  }
  if (!args.input) die("--input is required");
  if (["metrics", "dashboard", "gate"].includes(args.mode) && !args.baseline) die("--baseline is required for metrics/dashboard/gate");
  if (!["metrics", "dashboard", "gate"].includes(args.mode) && args.baseline) die("--baseline is only supported for metrics/dashboard/gate");
  if (!args.outDir) die("--out-dir is required");
  return args;
}

const args = parseArgs(process.argv);
if (args.mode === "gate") {
  const result = gate(args);
  const line = `${result.overall_status} phase 6 cloud eval monitoring gate: ${args.outDir}/release-gate.json`;
  if (result.overall_status === "BLOCK") {
    console.error(line);
    process.exit(65);
  }
  console.log(line);
  process.exit(0);
}

if (args.mode === "dashboard") {
  const result = dashboard(args);
  if (result.status !== "PASS") {
    const names = result.errors.map((error) => `${error.name}:${error.detail}`).join(",");
    console.error(`FAIL phase 6 cloud eval monitoring dashboard: ${names}`);
    process.exit(64);
  }
  console.log(`PASS phase 6 cloud eval monitoring dashboard: ${args.outDir}/cloud-eval-monitoring-report.json`);
  process.exit(0);
}

if (args.mode === "metrics") {
  const result = metrics(args);
  if (result.status !== "PASS") {
    const names = result.errors.map((error) => `${error.name}:${error.detail}`).join(",");
    console.error(`FAIL phase 6 cloud eval monitoring metrics: ${names}`);
    process.exit(64);
  }
  console.log(`PASS phase 6 cloud eval monitoring metrics: ${args.outDir}/metrics.json`);
  process.exit(0);
}

if (args.mode === "generate") {
  const result = generate(args);
  if (result.status !== "PASS") {
    const names = result.errors.map((error) => `${error.name}:${error.detail}`).join(",");
    console.error(`FAIL phase 6 cloud eval monitoring generate: ${names}`);
    process.exit(64);
  }
  console.log(`PASS phase 6 cloud eval monitoring generate: ${args.outDir}/aggregation-summary.json`);
  process.exit(0);
}

const result = validateInputFixture(args.input);
if (result.validation) writeInputValidation(args.outDir, result.validation);
if (result.status !== "PASS") {
  const names = [...new Set(result.errors.map((error) => error.name))].join(",");
  console.error(`FAIL phase 6 cloud eval monitoring validate-input: ${names}`);
  process.exit(64);
}
console.log(`PASS phase 6 cloud eval monitoring validate-input: ${args.outDir}/input-validation.json`);
