import { execFileSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

import { REPORT_JSON, REPORT_MD } from "./preflight-constants.mjs";

export function readText(repoRoot, relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), "utf8");
}

export function readJson(repoRoot, relativePath) {
  return JSON.parse(readText(repoRoot, relativePath));
}

export function sourceReceipt(repoRoot, relativePath) {
  const absolutePath = path.join(repoRoot, relativePath);
  if (!fs.existsSync(absolutePath)) {
    return { path: relativePath, exists: false };
  }
  const bytes = fs.readFileSync(absolutePath);
  const stat = fs.statSync(absolutePath);
  return {
    path: relativePath,
    exists: true,
    bytes: stat.size,
    mtime_ms: stat.mtimeMs,
    sha256: crypto.createHash("sha256").update(bytes).digest("hex")
  };
}

export function git(args) {
  return execFileSync("git", args, { encoding: "utf8" });
}

export function writeReports(outDir, report, markdown) {
  fs.mkdirSync(outDir, { recursive: true });
  fs.writeFileSync(path.join(outDir, REPORT_JSON), `${JSON.stringify(report, null, 2)}\n`);
  fs.writeFileSync(path.join(outDir, REPORT_MD), markdown);
}
