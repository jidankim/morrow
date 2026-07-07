#!/usr/bin/env node
import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { createReadStream } from "node:fs";
import { mkdir, readdir, readFile, stat, writeFile } from "node:fs/promises";
import path from "node:path";
import { promisify } from "node:util";

const run = promisify(execFile);
const [manifestPath, bundleDir, evidencePath] = process.argv.slice(2);
const checks = [];
const artifacts = { apps: [], dmgs: [] };

function record(name, passed, details) {
  checks.push({ status: passed ? "PASS" : "FAIL", name, details });
}

async function exists(filePath) {
  try {
    return await stat(filePath);
  } catch {
    return undefined;
  }
}

async function walk(dir) {
  let entries;
  try {
    entries = await readdir(dir, { withFileTypes: true });
  } catch {
    return;
  }
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory() && entry.name === "Morrow.app") {
      artifacts.apps.push(fullPath);
      continue;
    }
    if (entry.isFile() && entry.name.endsWith(".dmg")) {
      artifacts.dmgs.push(fullPath);
      continue;
    }
    if (entry.isDirectory() && !entry.name.endsWith(".app")) {
      await walk(fullPath);
    }
  }
}

async function hashFile(filePath) {
  const hash = createHash("sha256");
  await new Promise((resolve, reject) => {
    createReadStream(filePath).on("data", (chunk) => hash.update(chunk)).on("error", reject).on("end", resolve);
  });
  return hash.digest("hex");
}

async function hashDirectory(dir) {
  const files = [];
  async function collect(current) {
    const entries = await readdir(current, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        await collect(fullPath);
      } else if (entry.isFile()) {
        files.push(fullPath);
      }
    }
  }
  await collect(dir);
  files.sort();
  const hash = createHash("sha256");
  for (const file of files) {
    hash.update(path.relative(dir, file));
    hash.update("\0");
    hash.update(await hashFile(file));
    hash.update("\0");
  }
  return hash.digest("hex");
}

async function gitValue(args) {
  try {
    const { stdout } = await run("git", args, { timeout: 10_000 });
    return stdout.trim();
  } catch {
    return "";
  }
}

function includesAny(text, phrases) {
  return phrases.some((phrase) => text.includes(phrase));
}

async function main() {
  if (!manifestPath || !bundleDir || !evidencePath) {
    console.error("Usage: node scripts/release-manifest-qa.mjs <manifest.md> <bundle-dir> <evidence.md>");
    process.exitCode = 2;
    return;
  }

  let manifest = "";
  try {
    manifest = await readFile(manifestPath, "utf8");
    record("release manifest is readable", true, manifestPath);
    record("release manifest is not empty", manifest.trim().length > 0, `${manifest.length} bytes`);
  } catch (error) {
    record("release manifest is readable", false, `${manifestPath}: ${error.message}`);
    record("release manifest is not empty", false, "manifest could not be read");
  }

  const bundleInfo = await exists(bundleDir);
  record("bundle directory exists", Boolean(bundleInfo?.isDirectory()), bundleDir);
  if (bundleInfo?.isDirectory()) {
    await walk(bundleDir);
  }
  record("exactly one Morrow.app bundle is present", artifacts.apps.length === 1, `found ${artifacts.apps.length}`);
  record("exactly one DMG is present", artifacts.dmgs.length === 1, `found ${artifacts.dmgs.length}`);

  const branch = await gitValue(["rev-parse", "--abbrev-ref", "HEAD"]);
  const commit = await gitValue(["rev-parse", "HEAD"]);
  if (branch) {
    record("manifest records current branch", manifest.includes(branch), branch);
  }
  if (commit) {
    record("manifest records current commit SHA", manifest.includes(commit), commit);
  }

  for (const app of artifacts.apps) {
    const digest = await hashDirectory(app);
    record("manifest records app artifact path", manifest.includes(app), app);
    record("manifest records app SHA-256", manifest.includes(digest), digest);
  }
  for (const dmg of artifacts.dmgs) {
    const digest = await hashFile(dmg);
    record("manifest records DMG artifact path", manifest.includes(dmg), dmg);
    record("manifest records DMG SHA-256", manifest.includes(digest), digest);
  }

  record("manifest records signing status", /sign(ed|ing)/i.test(manifest), "expected signing status language");
  record("manifest records notarization status", /notari[sz](ed|ation)/i.test(manifest), "expected notarization status language");
  record("manifest records stapling status", /stapl(e|ed|ing|er)/i.test(manifest), "expected stapling status language");

  for (const evidence of [
    ".omo/evidence/task-1-beta-release-distribution.md",
    ".omo/evidence/task-2-beta-release-distribution.md",
    ".omo/evidence/task-3-beta-release-distribution.md",
    ".omo/evidence/task-4-beta-release-distribution.md",
    ".omo/evidence/task-5-beta-release-distribution.md",
    ".omo/evidence/task-6-beta-release-distribution.md",
    ".omo/evidence/task-7-beta-release-distribution.md",
    ".omo/evidence/task-8-beta-release-distribution.md",
  ]) {
    record(`manifest records QA evidence path ${evidence}`, manifest.includes(evidence), evidence);
  }

  record(
    "manifest states package QA limitation",
    includesAny(manifest, [
      "does not prove real Messages-to-Calendar",
      "does not prove real Messages to Calendar",
      "package QA does not prove real",
    ]),
    "expected limitation that package QA does not prove real Messages-to-Calendar event creation",
  );
  record(
    "manifest does not claim package launch proves real QA",
    !manifest.includes("package launch proves real Messages-to-Calendar"),
    "forbidden overclaim absent",
  );

  const lines = [
    "# Task 9 Beta Release Manifest QA",
    "",
    "## Invocation",
    "",
    `\`node ${process.argv.slice(1).map((value) => JSON.stringify(value)).join(" ")}\``,
    "",
    "## Inputs",
    "",
    `- Manifest: \`${manifestPath}\``,
    `- Bundle directory: \`${bundleDir}\``,
    `- Evidence path: \`${evidencePath}\``,
    `- Branch: \`${branch || "(unavailable)"}\``,
    `- Commit: \`${commit || "(unavailable)"}\``,
    "",
    "## Artifacts",
    "",
    `- Morrow.app bundles: ${artifacts.apps.length}`,
    ...artifacts.apps.map((item) => `  - \`${item}\``),
    `- DMG files: ${artifacts.dmgs.length}`,
    ...artifacts.dmgs.map((item) => `  - \`${item}\``),
    "",
    "## Assertions",
    "",
    ...checks.map((check) => `- ${check.status} ${check.name}: ${check.details}`),
    "",
    checks.every((check) => check.status === "PASS") ? "RESULT: PASS" : "RESULT: FAIL",
    "",
  ];

  await mkdir(path.dirname(evidencePath), { recursive: true });
  await writeFile(evidencePath, `${lines.join("\n")}\n`, "utf8");
  for (const check of checks) {
    console.log(`${check.status} ${check.name}: ${check.details}`);
  }
  if (checks.some((check) => check.status !== "PASS")) {
    process.exitCode = 1;
  }
}

await main();
