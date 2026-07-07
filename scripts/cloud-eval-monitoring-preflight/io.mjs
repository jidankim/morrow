import { execFileSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";

export function runGit(args) {
  return execFileSync("git", args, { encoding: "utf8" });
}

export function exists(filePath) {
  return fs.existsSync(filePath);
}

export function readText(filePath) {
  return fs.readFileSync(filePath, "utf8");
}

export function readJson(filePath) {
  return JSON.parse(readText(filePath));
}

export function sha256(text) {
  return crypto.createHash("sha256").update(text).digest("hex");
}

export function sourceReceipt(rootLabel, relativePath, absolutePath) {
  if (!exists(absolutePath)) {
    return {
      root: rootLabel,
      path: relativePath,
      exists: false,
    };
  }
  const text = readText(absolutePath);
  const stat = fs.statSync(absolutePath);
  return {
    root: rootLabel,
    path: relativePath,
    exists: true,
    bytes: stat.size,
    mtime_ms: stat.mtimeMs,
    sha256: sha256(text),
  };
}

export function writeReportFiles(outDir, reportJsonName, reportMdName, report, markdown) {
  fs.mkdirSync(outDir, { recursive: true });
  fs.writeFileSync(pathJoin(outDir, reportJsonName), `${JSON.stringify(report, null, 2)}\n`);
  fs.writeFileSync(pathJoin(outDir, reportMdName), markdown);
}

function pathJoin(left, right) {
  return `${left.replace(/\/$/, "")}/${right}`;
}
