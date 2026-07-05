#!/usr/bin/env node
import { die } from "./human-approval-correction-smoke-report-lib.mjs";
import { generate } from "./human-approval-correction-smoke-report-generate.mjs";
import { validate } from "./human-approval-correction-smoke-report-validate.mjs";

function parseArgs(argv) {
  const args = { mode: argv[2], outDir: "", invocation: "", decisionEvidenceSource: "" };
  for (let index = 3; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--out-dir") {
      args.outDir = argv[index + 1] || "";
      index += 1;
    } else if (arg === "--invocation") {
      args.invocation = argv[index + 1] || "";
      index += 1;
    } else if (arg === "--decision-evidence-source") {
      args.decisionEvidenceSource = argv[index + 1] || "";
      index += 1;
    } else {
      die(`unknown argument: ${arg}`);
    }
  }
  if (!args.mode || !["generate", "validate"].includes(args.mode)) {
    die("usage: human-approval-correction-smoke-report.mjs <generate|validate> --out-dir <path>");
  }
  if (!args.outDir) die("--out-dir is required");
  if (args.mode === "generate" && !args.decisionEvidenceSource) {
    die("--decision-evidence-source is required in generate mode");
  }
  return args;
}

const args = parseArgs(process.argv);
if (args.mode === "generate") generate(args);
if (args.mode === "validate") {
  validate(args);
  console.log("PASS phase 4 human approval correction report validate");
}
