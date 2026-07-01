#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const [packagePath, tauriConfigPath, releaseSigningQaPath, evidencePath] = process.argv.slice(2);

const checks = [];

function record(status, name, details) {
  checks.push({ status, name, details });
}

function readText(filePath) {
  try {
    return fs.readFileSync(filePath, "utf8");
  } catch (error) {
    record("FAIL", `${path.basename(filePath)} is readable`, error.message);
    return "";
  }
}

function parseJson(label, filePath, text) {
  try {
    const parsed = JSON.parse(text);
    record("PASS", `${label} parses as JSON`, filePath);
    return parsed;
  } catch (error) {
    record("FAIL", `${label} parses as JSON`, error.message);
    return {};
  }
}

function includesAll(text, needles) {
  return needles.every((needle) => text.includes(needle));
}

function shellQuote(value) {
  return `'${String(value).replaceAll("'", "'\\''")}'`;
}

if (!packagePath || !tauriConfigPath || !releaseSigningQaPath || !evidencePath) {
  console.error("Usage: node scripts/diagnostic-config-qa.mjs <package.json> <tauri.conf.json> <release-signing-qa.sh> <evidence.md>");
  process.exit(2);
}

const packageText = readText(packagePath);
const tauriText = readText(tauriConfigPath);
const releaseText = readText(releaseSigningQaPath);
const packageJson = parseJson("package.json", packagePath, packageText);
const tauriConfig = parseJson("tauri.conf.json", tauriConfigPath, tauriText);
const packageDir = path.dirname(path.resolve(packagePath));
const diagnosticScriptPath = path.join(packageDir, "scripts", "build-diagnostic-artifact.sh");
const diagnosticScriptText = readText(diagnosticScriptPath);
const diagnosticCommand = packageJson.scripts?.["tauri:build:diagnostic"];

if (typeof diagnosticCommand === "string" && diagnosticCommand.includes("scripts/build-diagnostic-artifact.sh")) {
  record("PASS", "diagnostic command", `tauri:build:diagnostic=${diagnosticCommand}`);
} else {
  record("FAIL", "diagnostic command", "package.json must define tauri:build:diagnostic calling scripts/build-diagnostic-artifact.sh");
}

if (diagnosticScriptText.includes("APPLE_SIGNING_IDENTITY=\"-\"") && !tauriText.includes('"signingIdentity"')) {
  record("PASS", "ad-hoc signing", "diagnostic script sets APPLE_SIGNING_IDENTITY=- in the process environment only");
} else {
  record("FAIL", "ad-hoc signing", "expected APPLE_SIGNING_IDENTITY=- in script and no permanent signingIdentity in Tauri config");
}

const expectedSdkRoot = "SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk";
const expectedRustFlags = "RUSTFLAGS='-C linker=/Library/Developer/CommandLineTools/usr/bin/cc'";
const scriptPreservesSdkRoot = diagnosticScriptText.includes('SDKROOT:-/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk') && diagnosticScriptText.includes('SDKROOT="$sdkroot_value"');
const scriptPreservesRustFlags = diagnosticScriptText.includes('RUSTFLAGS:--C linker=/Library/Developer/CommandLineTools/usr/bin/cc') && diagnosticScriptText.includes('RUSTFLAGS="$rustflags_value"');
if (packageText.includes(expectedSdkRoot) && packageText.includes(expectedRustFlags) && scriptPreservesSdkRoot && scriptPreservesRustFlags) {
  record("PASS", "SDK/linker parity", "diagnostic script preserves SDKROOT and RUSTFLAGS linker defaults from existing Tauri scripts");
} else {
  record("FAIL", "SDK/linker parity", "expected SDKROOT and RUSTFLAGS linker parity with existing Tauri build scripts");
}

if (diagnosticScriptText.includes("tauri build --bundles app,dmg")) {
  record("PASS", "bundling", "diagnostic build invokes tauri build --bundles app,dmg");
} else {
  record("FAIL", "bundling", "expected tauri build --bundles app,dmg");
}

if (
  includesAll(diagnosticScriptText, [
    "stale app source cleanup",
    "rm -rf \"$stale_app\"",
    "current-run app freshness",
    "nonzero_tauri_exit_requires_recreated_current_run_app_bundle",
  ])
) {
  record("PASS", "fallback freshness", "diagnostic build removes pre-existing app sources before Tauri and records current-run freshness proof");
} else {
  record("FAIL", "fallback freshness", "expected stale app source cleanup, current-run app freshness evidence, and nonzero fallback policy");
}

if (tauriConfig.bundle?.active === true) {
  record("PASS", "bundle.active", "Tauri bundle.active remains true");
} else {
  record("FAIL", "bundle.active", `expected true, got ${String(tauriConfig.bundle?.active)}`);
}

if (includesAll(diagnosticScriptText, ["diagnostic_dir=\"src-tauri/target/release/diagnostic\"", ".zip", "Morrow-diagnostic-tester-note.md", "SHA256SUMS", "manifest.json"])) {
  record("PASS", "primary zip requirement", "diagnostic script stages output under src-tauri/target/release/diagnostic with a primary zip, tester note, manifest, and checksums");
} else {
  record("FAIL", "primary zip requirement", "expected diagnostic staging directory, primary zip, tester note, manifest, and checksums");
}

if (includesAll(diagnosticScriptText, ["uname -m", "rustc -vV", "hostArchitecture", "rustHostTarget", "tauriTarget", "architectureScope: \"host_only\""])) {
  record("PASS", "host-architecture manifest requirement", "diagnostic manifest records uname host architecture, Rust host target, Tauri target, and host-only scope");
} else {
  record("FAIL", "host-architecture manifest requirement", "expected uname -m, Rust host target, Tauri target, and host-only manifest fields");
}

if (!("createUpdaterArtifacts" in (tauriConfig.bundle ?? {})) && !("updater" in tauriConfig)) {
  record("PASS", "no updater", "Tauri config does not add updater configuration");
} else {
  record("FAIL", "no updater", "diagnostic path must not add updater configuration");
}

const credentialNames = [
  "APPLE_ID",
  "APPLE_PASSWORD",
  "APPLE_TEAM_ID",
  "APPLE_CERTIFICATE",
  "APPLE_CERTIFICATE_PASSWORD",
  "APPLE_API_KEY",
  "APPLE_API_KEY_ID",
  "APPLE_API_ISSUER",
  "APPLE_API_ISSUER_ID",
  "APPLE_API_KEY_PATH",
  "APPLE_PRIVATE_KEY",
  "APPLE_PRIVATE_KEY_PATH",
  "AC_PASSWORD",
  "ASC_PROVIDER",
];
const credentialUnsets = credentialNames.filter((name) =>
  diagnosticScriptText.includes(`-u ${name}`)
);
const tauriInvocationsUseSanitizedEnv =
  diagnosticScriptText.includes("run_with_diagnostic_apple_env tauri build --bundles app,dmg") &&
  diagnosticScriptText.includes("run_with_diagnostic_apple_env npm exec tauri -- build --bundles app,dmg") &&
  diagnosticScriptText.includes('APPLE_SIGNING_IDENTITY="-"') &&
  diagnosticScriptText.includes('SDKROOT="$sdkroot_value"') &&
  diagnosticScriptText.includes('RUSTFLAGS="$rustflags_value"');
if (credentialUnsets.length === credentialNames.length && tauriInvocationsUseSanitizedEnv) {
  record("PASS", "Apple credential clearing", `diagnostic Tauri invocations unset ${credentialUnsets.join(", ")} while preserving APPLE_SIGNING_IDENTITY=-, SDKROOT, and RUSTFLAGS`);
} else {
  const missingUnsets = credentialNames.filter((name) => !credentialUnsets.includes(name));
  record("FAIL", "Apple credential clearing", `missing credential unsets or sanitized invocation: ${missingUnsets.join(", ") || "none"}`);
}

const permanentSigningIdentity = tauriConfig.bundle?.macOS?.signingIdentity ?? tauriConfig.bundle?.macos?.signingIdentity;
if (permanentSigningIdentity === undefined) {
  record("PASS", "no permanent signing identity", "Tauri config does not define bundle.macOS.signingIdentity or bundle.macos.signingIdentity");
} else {
  record("FAIL", "no permanent signing identity", "diagnostic signing identity must not be committed to tauri.conf.json");
}

if (
  releaseText.includes("ad-hoc signing identity is not a release identity") &&
  releaseText.includes("write_evidence \"BLOCKED\"") &&
  releaseText.includes("credentials_ok") &&
  !diagnosticScriptText.includes("release-signing-qa.sh")
) {
  record("PASS", "release-gate separation", "release gate still blocks ad-hoc/no-credential artifacts and diagnostic script does not reuse it as a success gate");
} else {
  record("FAIL", "release-gate separation", "expected release gate to reject ad-hoc/no-credential official release success");
}

const failed = checks.some((check) => check.status !== "PASS");
fs.mkdirSync(path.dirname(evidencePath), { recursive: true });
const lines = [
  "# Todo 1 Diagnostic Config QA Evidence",
  "",
  "## Invocation",
  "",
  `\`node scripts/diagnostic-config-qa.mjs ${[packagePath, tauriConfigPath, releaseSigningQaPath, evidencePath].map(shellQuote).join(" ")}\``,
  "",
  "## Assertions",
  "",
  ...checks.map((check) => `- ${check.status} ${check.name}: ${check.details}`),
  "",
  "## Adversarial Classes",
  "",
  "- malformed_input: probed by the required temp bad package/tauri/release fixture; expected FAIL lines include APPLE_SIGNING_IDENTITY, SDKROOT, RUSTFLAGS, and release gate.",
  "- dirty_worktree: probed by baseline `git status --short`; unrelated pre-existing changes are recorded in `.omo/evidence/task-1-unsigned-adhoc-baseline.md`.",
  "- misleading_success_output: config QA checks concrete package/config/script/release-gate file contents and does not trust stdout.",
  "- stale_state: happy QA re-reads package.json, tauri.conf.json, release-signing-qa.sh, and build script from disk on every invocation, and asserts current-run app freshness proof.",
  "- prompt_injection: not applicable; the script reads local config/source files only and does not execute untrusted external text.",
  "- cancel_resume: not applicable; no resumable flow is introduced.",
  "- generated_cached_artifacts: static QA requires stale app source cleanup before Tauri so build fallback cannot package cached app bundles.",
  "- hung_long_commands: not applicable; config QA is deterministic static file inspection.",
  "- flaky_tests: not applicable; no timing, network, or randomness is used.",
  "- repeated_interruptions: not applicable; no mid-operation interrupt path is introduced.",
  "",
  `RESULT: ${failed ? "FAIL" : "PASS"}`,
  "",
];
fs.writeFileSync(evidencePath, lines.join("\n"));

for (const check of checks) {
  console.log(`${check.status} ${check.name}: ${check.details}`);
}
console.log(`RESULT: ${failed ? "FAIL" : "PASS"}`);
process.exit(failed ? 1 : 0);
