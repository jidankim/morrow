import { REQUIRED_KILL_SWITCHES } from "./report-constants.mjs";
import { buildInvocation, readJson, writeReport } from "./report-io.mjs";
import { check, validateManifestObject } from "./report-validation.mjs";

const ENABLEMENT_KEY = /(?:enable|allow|create|activate).*(?:telemetry|upload)|(?:telemetry|upload).*(?:enable|allow|capability)/i;
const ENABLEMENT_TEXT = /\b(?:enable[sd]?|allow[sed]?|creates?)\s+(?:telemetry|upload|an upload path)\b|telemetry becomes enabled/i;

function collectEnablementClaims(value, pointer = "$.kill_switches", findings = []) {
  if (Array.isArray(value)) {
    value.forEach((entry, index) => collectEnablementClaims(entry, `${pointer}[${index}]`, findings));
    return findings;
  }
  if (value && typeof value === "object") {
    for (const [key, entry] of Object.entries(value)) {
      const child = `${pointer}.${key}`;
      if (ENABLEMENT_KEY.test(key) && entry === true) findings.push({ pointer: child, class: "truthy_enablement_flag" });
      collectEnablementClaims(entry, child, findings);
    }
    return findings;
  }
  if (typeof value === "string" && ENABLEMENT_TEXT.test(value)) {
    findings.push({ pointer, class: "enablement_text" });
  }
  return findings;
}

function proofRows(manifest) {
  return [
    ["telemetry", "telemetry"],
    ["upload", "upload"],
    ["model/router changes", "model_router_changes"],
    ["provider path use", "provider_changes"],
    ["staged promotion", "staged_promotion"],
    ["default availability", "default_availability"],
  ].map(([capability, key]) => ({
    capability,
    kill_switch: key,
    enabled: manifest.kill_switches?.[key]?.enabled ?? null,
    fail_closed: manifest.kill_switches?.[key]?.fail_closed ?? null,
    observable: "enabled=false and fail_closed=true means the capability remains blocked",
  }));
}

export async function runKillSwitchCheck(args) {
  if (!args.manifest || !args.out_dir) {
    throw new Error("Usage: kill-switch-check --manifest <path> --out-dir <dir>");
  }

  const manifest = await readJson(args.manifest);
  const validation = validateManifestObject(manifest);
  const checks = [...validation.checks];
  const switches = manifest.kill_switches ?? {};
  const enablementFindings = collectEnablementClaims(switches);

  check(checks, "telemetry is hard-off at manifest root", manifest.telemetry_enabled === false, `observed ${JSON.stringify(manifest.telemetry_enabled)}`);
  check(checks, "upload cannot be enabled by telemetry switch", enablementFindings.length === 0, `observed ${JSON.stringify(enablementFindings)}`);
  for (const key of REQUIRED_KILL_SWITCHES) {
    check(checks, `kill switch ${key} blocks its capability`, switches[key]?.enabled === false && switches[key]?.fail_closed === true, `observed ${JSON.stringify(switches[key])}`);
  }

  const status = checks.every((entry) => entry.status === "PASS") ? "PASS" : "FAIL";
  const report = {
    mode: "kill-switch-check",
    status,
    validated_at: new Date().toISOString(),
    invocation: buildInvocation(),
    manifest_path: args.manifest,
    no_telemetry_or_upload_enablement_claims: enablementFindings.length === 0,
    fail_closed_capability_proofs: proofRows(manifest),
    checks,
  };
  const reportPath = await writeReport(args.out_dir, "kill-switches.json", report);
  console.log(`${status} kill-switch check: ${reportPath}`);
  if (status !== "PASS") process.exitCode = 1;
}
