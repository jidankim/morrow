#!/usr/bin/env node
import {
  ALLOWED_STATES,
  ALLOWED_TRANSITIONS,
  FAIL_CLOSED_STATES,
  OUT_OF_SCOPE_MODES,
} from "./wider-product-rollout/report-constants.mjs";
import { runGate } from "./wider-product-rollout/report-gate.mjs";
import { runVerifyHandoff } from "./wider-product-rollout/report-handoff.mjs";
import {
  buildInvocation,
  parseArgs,
  readJson,
  writeReport,
} from "./wider-product-rollout/report-io.mjs";
import { runKillSwitchCheck } from "./wider-product-rollout/report-kill-switches.mjs";
import { runPrivacyScan } from "./wider-product-rollout/report-privacy.mjs";
import {
  check,
  validateManifestObject,
} from "./wider-product-rollout/report-validation.mjs";

async function validateManifest(args) {
  if (!args.manifest || !args.out_dir) {
    throw new Error("Usage: validate-manifest --manifest <path> --out-dir <dir>");
  }

  let manifest;
  const parseChecks = [];
  try {
    manifest = await readJson(args.manifest);
    check(parseChecks, "manifest parses as JSON", true, args.manifest);
  } catch (error) {
    check(parseChecks, "manifest parses as JSON", false, `${args.manifest}: ${error.message}`);
    manifest = {};
  }

  const validation = validateManifestObject(manifest);
  const checks = [...parseChecks, ...validation.checks];
  const status = checks.every((entry) => entry.status === "PASS") ? "PASS" : "FAIL";
  const report = {
    mode: "validate-manifest",
    status,
    validated_at: new Date().toISOString(),
    invocation: buildInvocation(),
    manifest_path: args.manifest,
    output_dir: args.out_dir,
    rollout_source_of_truth: manifest.rollout_source_of_truth ?? null,
    remote_config_enabled: manifest.remote_config_enabled ?? null,
    telemetry_enabled: manifest.telemetry_enabled ?? null,
    state: manifest.state ?? null,
    allowed_states: [...ALLOWED_STATES],
    kill_switch_fields_present: validation.killSwitchFieldsPresent,
    no_credentials_private_paths_or_raw_content: validation.forbiddenEntries.length === 0,
    default_availability_scope:
      manifest.default_availability?.default_download_channel ?? null,
    checks,
  };

  const reportPath = await writeReport(args.out_dir, "manifest-validation.json", report);
  console.log(`${status} manifest validation: ${reportPath}`);
  if (status !== "PASS") {
    process.exitCode = 1;
  }
}

async function validateTransition(args) {
  if (!args.from || !args.to || !args.out_dir) {
    throw new Error("Usage: validate-transition --from <state> --to <state> --out-dir <dir>");
  }

  const checks = [];
  check(checks, "from state is allowed", ALLOWED_STATES.has(args.from), `observed ${JSON.stringify(args.from)}`);
  check(checks, "to state is allowed", ALLOWED_STATES.has(args.to), `observed ${JSON.stringify(args.to)}`);
  check(
    checks,
    "fail-closed states cannot promote",
    !FAIL_CLOSED_STATES.has(args.from),
    `observed from=${JSON.stringify(args.from)}`,
  );

  const transition = `${args.from}->${args.to}`;
  check(
    checks,
    "transition is an allowed forward promotion",
    ALLOWED_TRANSITIONS.has(transition),
    `observed ${transition}; allowed ${JSON.stringify([...ALLOWED_TRANSITIONS])}`,
  );

  const status = checks.every((entry) => entry.status === "PASS") ? "PASS" : "FAIL";
  const report = {
    mode: "validate-transition",
    status,
    validated_at: new Date().toISOString(),
    invocation: buildInvocation(),
    from: args.from,
    to: args.to,
    output_dir: args.out_dir,
    allowed_forward_transitions: [...ALLOWED_TRANSITIONS],
    fail_closed_states: [...FAIL_CLOSED_STATES],
    checks,
  };

  const reportPath = await writeReport(args.out_dir, "transition-validation.json", report);
  console.log(`${status} transition validation: ${reportPath}`);
  if (status !== "PASS") {
    process.exitCode = 1;
  }
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.mode) {
    throw new Error(
      "Usage: node scripts/wider-product-rollout-report.mjs <validate-manifest|validate-transition> ...",
    );
  }

  if (args.mode === "validate-manifest") {
    await validateManifest(args);
    return;
  }
  if (args.mode === "validate-transition") {
    await validateTransition(args);
    return;
  }
  if (args.mode === "gate") {
    await runGate(args);
    return;
  }
  if (args.mode === "privacy-scan") {
    await runPrivacyScan(args);
    return;
  }
  if (args.mode === "kill-switch-check") {
    await runKillSwitchCheck(args);
    return;
  }
  if (args.mode === "verify-handoff") {
    await runVerifyHandoff(args);
    return;
  }
  if (OUT_OF_SCOPE_MODES.has(args.mode)) {
    throw new Error(`mode ${args.mode} is reserved for later rollout todos and is not implemented by Todo 2`);
  }

  throw new Error(`unknown mode: ${args.mode}`);
}

try {
  await main();
} catch (error) {
  console.error(`FAIL ${error.message}`);
  process.exitCode = 2;
}
