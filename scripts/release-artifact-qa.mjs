#!/usr/bin/env node
import { execFile } from "node:child_process";
import { mkdir, readFile, readdir, stat, writeFile } from "node:fs/promises";
import path from "node:path";
import { promisify } from "node:util";

const run = promisify(execFile);
const [bundleDir, evidencePath] = process.argv.slice(2);
const tauriConfigPath = path.join(process.cwd(), "src-tauri", "tauri.conf.json");
const checks = [];

function record(name, passed, details) {
  checks.push({ name, status: passed ? "PASS" : "FAIL", details });
}

function observed(value) {
  return `observed ${JSON.stringify(value)}`;
}

function recordObserved(name, passed, value) {
  record(name, passed, observed(value));
}

function recordSingle(name, count) {
  record(name, count === 1, `found ${count}`);
}

async function readJson(label, filePath) {
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

async function pathExists(filePath) {
  try {
    await stat(filePath);
    return true;
  } catch {
    return false;
  }
}

async function isFile(filePath) {
  try {
    const info = await stat(filePath);
    return info.isFile();
  } catch {
    return false;
  }
}

async function findArtifacts(root) {
  const appBundles = [];
  const dmgs = [];

  async function walk(dir) {
    let entries;
    try {
      entries = await readdir(dir, { withFileTypes: true });
    } catch (error) {
      record("bundle directory is readable", false, `${dir}: ${error.message}`);
      return;
    }

    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);

      if (entry.isDirectory() && entry.name === "Morrow.app") {
        appBundles.push(fullPath);
        continue;
      }

      if (entry.isFile() && path.extname(entry.name) === ".dmg") {
        dmgs.push(fullPath);
        continue;
      }

      if (entry.isDirectory() && !entry.name.endsWith(".app")) {
        await walk(fullPath);
      }
    }
  }

  if (await pathExists(root)) {
    record("bundle directory exists", true, root);
    await walk(root);
  } else {
    record("bundle directory exists", false, `${root} does not exist`);
  }

  return { appBundles, dmgs };
}

async function readPlist(plistPath) {
  try {
    const { stdout } = await run("plutil", ["-convert", "json", "-o", "-", plistPath], {
      timeout: 10_000,
      maxBuffer: 1024 * 1024,
    });
    record("Info.plist converts to JSON with plutil", true, plistPath);
    try {
      const parsed = JSON.parse(stdout);
      record("Info.plist JSON parses", true, plistPath);
      return parsed;
    } catch (error) {
      record("Info.plist JSON parses", false, `${plistPath}: ${error.message}`);
      return undefined;
    }
  } catch (error) {
    const message = error.stderr || error.message;
    record("Info.plist converts to JSON with plutil", false, `${plistPath}: ${message}`);
    record("Info.plist JSON parses", false, "plutil conversion failed");
    return undefined;
  }
}

function buildInvocation() {
  return `node ${process.argv.slice(1).map((value) => JSON.stringify(value)).join(" ")}`;
}

function buildEvidence(invocation, artifacts, tauriConfig) {
  const lines = [
    "# Task 2 Beta Release Distribution Evidence",
    "",
    "## Invocation",
    "",
    `\`${invocation}\``,
    "",
    "## Inputs",
    "",
    `- Bundle directory: \`${bundleDir ?? "(missing)"}\``,
    `- Evidence path: \`${evidencePath ?? "(missing)"}\``,
    `- Tauri config: \`${tauriConfigPath}\``,
    `- Config version observed: \`${tauriConfig?.version ?? "(unavailable)"}\``,
    "",
    "## Discovered Artifacts",
    "",
    `- Morrow.app bundles: ${artifacts.appBundles.length}`,
    ...artifacts.appBundles.map((appPath) => `  - \`${appPath}\``),
    `- DMG files: ${artifacts.dmgs.length}`,
    ...artifacts.dmgs.map((dmgPath) => `  - \`${dmgPath}\``),
    "",
    "## Assertions",
    "",
  ];

  for (const check of checks) {
    lines.push(`- ${check.status} ${check.name}: ${check.details}`);
  }

  lines.push("", checks.every((check) => check.status === "PASS") ? "RESULT: PASS" : "RESULT: FAIL", "");

  return `${lines.join("\n")}\n`;
}

async function main() {
  if (!bundleDir || !evidencePath) {
    console.error(
      "Usage: node scripts/release-artifact-qa.mjs <bundle-dir> <evidence.md>",
    );
    process.exitCode = 2;
    return;
  }

  const tauriConfig = await readJson("tauri config", tauriConfigPath);
  const expectedVersion =
    typeof tauriConfig?.version === "string" && tauriConfig.version.length > 0
      ? tauriConfig.version
      : undefined;

  recordObserved(
    "tauri identifier is dev.morrow.desktop",
    tauriConfig?.identifier === "dev.morrow.desktop",
    tauriConfig?.identifier,
  );
  recordObserved(
    "tauri productName is Morrow",
    tauriConfig?.productName === "Morrow",
    tauriConfig?.productName,
  );
  record(
    "tauri version is available",
    expectedVersion !== undefined,
    expectedVersion === undefined ? observed(tauriConfig?.version) : `expected version ${expectedVersion}`,
  );

  const artifacts = await findArtifacts(bundleDir);
  recordSingle("exactly one Morrow.app bundle is present", artifacts.appBundles.length);
  recordSingle("exactly one DMG is present", artifacts.dmgs.length);

  const appBundle = artifacts.appBundles[0];
  const dmg = artifacts.dmgs[0];

  if (appBundle) {
    const plistPath = path.join(appBundle, "Contents", "Info.plist");
    record("Info.plist exists", await isFile(plistPath), plistPath);
    const plist = await readPlist(plistPath);

    recordObserved(
      "CFBundleIdentifier is dev.morrow.desktop",
      plist?.CFBundleIdentifier === "dev.morrow.desktop",
      plist?.CFBundleIdentifier,
    );

    const bundleName = plist?.CFBundleName;
    const displayName = plist?.CFBundleDisplayName;
    recordObserved("CFBundleName is Morrow", bundleName === "Morrow", bundleName);
    recordObserved(
      "CFBundleDisplayName is absent or Morrow",
      displayName === undefined || displayName === "Morrow",
      displayName,
    );

    const executableName = plist?.CFBundleExecutable;
    const executablePath =
      typeof executableName === "string"
        ? path.join(appBundle, "Contents", "MacOS", executableName)
        : undefined;

    recordObserved(
      "CFBundleExecutable is declared",
      typeof executableName === "string" && executableName.length > 0,
      executableName,
    );
    record(
      "CFBundleExecutable exists under Contents/MacOS",
      executablePath ? await isFile(executablePath) : false,
      executablePath ?? "missing CFBundleExecutable",
    );
  } else {
    record("Info.plist exists", false, "no Morrow.app bundle found");
    record("Info.plist converts to JSON with plutil", false, "no Morrow.app bundle found");
    record("Info.plist JSON parses", false, "no Morrow.app bundle found");
    record("CFBundleIdentifier is dev.morrow.desktop", false, "no Morrow.app bundle found");
    record("CFBundleName is Morrow", false, "no Morrow.app bundle found");
    record("CFBundleDisplayName is absent or Morrow", false, "no Morrow.app bundle found");
    record("CFBundleExecutable is declared", false, "no Morrow.app bundle found");
    record(
      "CFBundleExecutable exists under Contents/MacOS",
      false,
      "no Morrow.app bundle found",
    );
  }

  if (dmg) {
    const dmgName = path.basename(dmg);
    record(
      "DMG filename includes release version",
      expectedVersion !== undefined && dmgName.includes(expectedVersion),
      `${observed(dmgName)}, expected version ${expectedVersion ?? "(unavailable)"}`,
    );
  } else {
    record("DMG filename includes release version", false, "no DMG found");
  }

  const invocation = buildInvocation();
  await mkdir(path.dirname(evidencePath), { recursive: true });
  await writeFile(evidencePath, buildEvidence(invocation, artifacts, tauriConfig), "utf8");

  for (const check of checks) {
    console.log(`${check.status} ${check.name}: ${check.details}`);
  }

  if (checks.some((check) => check.status !== "PASS")) {
    process.exitCode = 1;
  }
}

await main();
