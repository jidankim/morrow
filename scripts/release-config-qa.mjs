#!/usr/bin/env node
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const [tauriConfigPath, packageJsonPath, evidencePath] = process.argv.slice(2);

const checks = [];

function record(name, passed, details) {
  checks.push({
    name,
    status: passed ? "PASS" : "FAIL",
    details,
  });
}

async function readJson(label, filePath) {
  if (!filePath) {
    record(`${label} path provided`, false, "missing CLI argument");
    return undefined;
  }

  try {
    const source = await readFile(filePath, "utf8");
    try {
      const parsed = JSON.parse(source);
      record(`${label} parses as JSON`, true, filePath);
      return parsed;
    } catch (error) {
      record(`${label} parses as JSON`, false, `${filePath}: ${error.message}`);
      return undefined;
    }
  } catch (error) {
    record(`${label} file is readable`, false, `${filePath}: ${error.message}`);
    return undefined;
  }
}

function hasOwn(object, key) {
  return Object.prototype.hasOwnProperty.call(object, key);
}

function betaBuildScript(packageJson) {
  const scripts = packageJson?.scripts;
  if (!scripts || typeof scripts !== "object" || Array.isArray(scripts)) {
    return undefined;
  }

  const preferred = scripts["tauri:build:beta"];
  if (typeof preferred === "string") {
    return {
      name: "tauri:build:beta",
      command: preferred,
    };
  }

  const fallback = Object.entries(scripts).find(
    ([name, command]) =>
      name.includes("beta") &&
      typeof command === "string" &&
      command.includes("tauri build --bundles app,dmg"),
  );

  if (!fallback) {
    return undefined;
  }

  return {
    name: fallback[0],
    command: fallback[1],
  };
}

function buildEvidence(invocation, tauriConfigPathArg, packageJsonPathArg) {
  const lines = [
    "# Task 1 Beta Release Distribution Evidence",
    "",
    "## Invocation",
    "",
    `\`${invocation}\``,
    "",
    "## Inputs",
    "",
    `- Tauri config: \`${tauriConfigPathArg ?? "(missing)"}\``,
    `- Package manifest: \`${packageJsonPathArg ?? "(missing)"}\``,
    "",
    "## Assertions",
    "",
  ];

  for (const check of checks) {
    lines.push(`- ${check.status} ${check.name}: ${check.details}`);
  }

  lines.push("");
  lines.push(
    checks.every((check) => check.status === "PASS")
      ? "RESULT: PASS"
      : "RESULT: FAIL",
  );
  lines.push("");

  return `${lines.join("\n")}\n`;
}

async function main() {
  if (!evidencePath) {
    console.error(
      "Usage: node scripts/release-config-qa.mjs <tauri.conf.json> <package.json> <evidence.md>",
    );
    process.exitCode = 2;
    return;
  }

  const [tauriConfig, packageJson] = await Promise.all([
    readJson("tauri config", tauriConfigPath),
    readJson("package manifest", packageJsonPath),
  ]);

  const bundle = tauriConfig?.bundle;
  record(
    "bundle.active is true",
    bundle?.active === true,
    `observed ${JSON.stringify(bundle?.active)}`,
  );
  record(
    "productName is Morrow",
    tauriConfig?.productName === "Morrow",
    `observed ${JSON.stringify(tauriConfig?.productName)}`,
  );
  record(
    "identifier is dev.morrow.desktop",
    tauriConfig?.identifier === "dev.morrow.desktop",
    `observed ${JSON.stringify(tauriConfig?.identifier)}`,
  );
  record(
    "bundle.createUpdaterArtifacts is absent",
    Boolean(bundle) && !hasOwn(bundle, "createUpdaterArtifacts"),
    `observed ${JSON.stringify(bundle?.createUpdaterArtifacts)}`,
  );

  const betaScript = betaBuildScript(packageJson);
  record(
    "package has tauri:build:beta script",
    betaScript?.name === "tauri:build:beta",
    betaScript ? `observed ${betaScript.name}` : "missing beta build script",
  );
  record(
    "beta build script keeps SDKROOT environment",
    typeof betaScript?.command === "string" && betaScript.command.includes("SDKROOT="),
    `observed ${JSON.stringify(betaScript?.command)}`,
  );
  record(
    "beta build script keeps RUSTFLAGS linker environment",
    typeof betaScript?.command === "string" &&
      betaScript.command.includes("RUSTFLAGS=") &&
      betaScript.command.includes("-C linker=/Library/Developer/CommandLineTools/usr/bin/cc"),
    `observed ${JSON.stringify(betaScript?.command)}`,
  );
  record(
    "beta build script invokes app and dmg bundles",
    typeof betaScript?.command === "string" &&
      betaScript.command.includes("tauri build --bundles app,dmg"),
    `observed ${JSON.stringify(betaScript?.command)}`,
  );

  const invocation = `node ${process.argv
    .slice(1)
    .map((value) => JSON.stringify(value))
    .join(" ")}`;
  await mkdir(path.dirname(evidencePath), { recursive: true });
  await writeFile(
    evidencePath,
    buildEvidence(invocation, tauriConfigPath, packageJsonPath),
    "utf8",
  );

  for (const check of checks) {
    console.log(`${check.status} ${check.name}: ${check.details}`);
  }

  if (checks.some((check) => check.status !== "PASS")) {
    process.exitCode = 1;
  }
}

await main();
