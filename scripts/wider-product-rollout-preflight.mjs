#!/usr/bin/env node

import path from "node:path";
import process from "node:process";

import { forbiddenContentScan } from "./wider-product-rollout/preflight-checks.mjs";
import { buildMarkdown } from "./wider-product-rollout/preflight-markdown.mjs";
import { buildReport } from "./wider-product-rollout/preflight-report.mjs";
import { writeReports } from "./wider-product-rollout/preflight-io.mjs";

const USAGE = "Usage: node scripts/wider-product-rollout-preflight.mjs --out-dir <dir> [--fixture <path>]";

function readOptionValue(argv, index, optionName) {
  const value = argv[index + 1];
  if (!value || value.startsWith("--")) {
    throw new Error(`${optionName} requires a value.\n${USAGE}`);
  }
  return value;
}

function parseArgs(argv) {
  const args = { outDir: undefined, fixture: undefined };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--out-dir") {
      args.outDir = readOptionValue(argv, index, "--out-dir");
      index += 1;
    } else if (arg === "--fixture") {
      args.fixture = readOptionValue(argv, index, "--fixture");
      index += 1;
    } else {
      throw new Error(`Unknown argument: ${arg}\n${USAGE}`);
    }
  }
  if (!args.outDir) {
    throw new Error(`Missing required --out-dir\n${USAGE}`);
  }
  return args;
}

function main() {
  const repoRoot = process.cwd();
  const args = parseArgs(process.argv.slice(2));
  const outDir = path.resolve(repoRoot, args.outDir);
  let report = buildReport(repoRoot, args);
  let markdown = buildMarkdown(report);
  const scan = forbiddenContentScan(report, markdown);
  report = {
    ...report,
    status: report.status === "FAIL" || scan.status === "FAIL" ? "FAIL" : report.status,
    forbidden_raw_content: scan
  };
  markdown = buildMarkdown(report);
  writeReports(outDir, report, markdown);

  if (report.status === "FAIL") {
    const contradiction = report.contradiction_fixture.contradictions[0]?.name;
    const suffix = contradiction ? ` contradiction=${contradiction}` : "";
    console.error(`FAIL Phase 7 preflight${suffix}: Phase 6 must remain fixture-only, not deployed rollout evidence.`);
    process.exit(2);
  }

  console.log(`PASS Phase 7 wider product rollout preflight: ${path.join(args.outDir, "preflight-report.json")}`);
}

try {
  main();
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`FAIL Phase 7 preflight: ${message}`);
  process.exit(1);
}
