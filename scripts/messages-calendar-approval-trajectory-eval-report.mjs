#!/usr/bin/env node
import { die } from "./messages-calendar-approval-trajectory-eval-report-lib.mjs";
import { generate } from "./messages-calendar-approval-trajectory-eval-report-generate.mjs";
import { validate } from "./messages-calendar-approval-trajectory-eval-report-validate.mjs";

function parseArgs(argv) {
  const args = { mode: argv[2], outDir: "", invocation: "", localRunSource: "" };
  for (let index = 3; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--out-dir") {
      args.outDir = argv[index + 1] || "";
      index += 1;
    } else if (arg === "--invocation") {
      args.invocation = argv[index + 1] || "";
      index += 1;
    } else if (arg === "--local-run-source") {
      args.localRunSource = argv[index + 1] || "";
      index += 1;
    } else {
      die(`unknown argument: ${arg}`);
    }
  }
  if (!args.mode || !["generate", "validate"].includes(args.mode)) {
    die("usage: messages-calendar-approval-trajectory-eval-report.mjs <generate|validate> --out-dir <path>");
  }
  if (!args.outDir) die("--out-dir is required");
  if (args.mode === "generate" && !args.localRunSource) {
    die("--local-run-source is required in generate mode");
  }
  return args;
}

const args = parseArgs(process.argv);
if (args.mode === "generate") generate(args);
if (args.mode === "validate") {
  validate(args);
  console.log("PASS phase 5 messages calendar approval trajectory eval report validate");
}
