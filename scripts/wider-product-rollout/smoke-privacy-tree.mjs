import fs from "node:fs";
import path from "node:path";

const forbiddenArtifact = /MORROW_PRIVACY_CANARY_RAW|PHASE6_RAW_CONTENT_CANARY_DO_NOT_STORE|\/Users\/|\/private\/|\/var\/folders\/|home\/\.codex\/auth\.json|~\/\.codex\/auth\.json|codex_access_token|CODEX_ACCESS_TOKEN|auth\.json|sk-[A-Za-z0-9]{12,}/;
const forbiddenBytes = [
  "/Users/",
  "/private/",
  "/var/folders/",
  "home/.codex/auth.json",
  "~/.codex/auth.json",
  "auth.json",
  "codex_access_token",
  "CODEX_ACCESS_TOKEN",
  "MORROW_PRIVACY_CANARY_RAW",
  "PHASE6_RAW_CONTENT_CANARY_DO_NOT_STORE",
];

function isText(buffer) {
  return !buffer.includes(0);
}

function normalizeRelative(root, filePath) {
  return path.relative(root, filePath).split(path.sep).join("/");
}

function walkTree(dir) {
  const entries = [];
  if (!fs.existsSync(dir)) return entries;
  const visit = (current) => {
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const fullPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        entries.push({ type: "directory", path: fullPath });
        visit(fullPath);
      } else if (entry.isFile()) {
        entries.push({ type: "file", path: fullPath });
      }
    }
  };
  visit(dir);
  return entries;
}

export function sanitizeText(text) {
  return text
    .split(process.cwd()).join("<repo>")
    .replaceAll("/Users/", "<users>/")
    .replaceAll("/private/", "<private>/")
    .replaceAll("/var/folders/", "<var_folders>/")
    .replaceAll("home/.codex/auth.json", "<credential-path>")
    .replaceAll("~/.codex/auth.json", "<credential-path>")
    .replaceAll("auth.json", "<credential-file>")
    .replaceAll("codex_access_token", "<credential-token-key>")
    .replaceAll("CODEX_ACCESS_TOKEN", "<credential-token-key>")
    .replaceAll("MORROW_PRIVACY_CANARY_RAW", "<privacy-canary>")
    .replaceAll("PHASE6_RAW_CONTENT_CANARY_DO_NOT_STORE", "<phase6-raw-content-canary>")
    .replace(/sk-[A-Za-z0-9]{12,}/g, "<secret-token>");
}

export function removeEphemeralRuntimeTrees(outDir) {
  let removed = 0;
  const entries = walkTree(outDir).filter((entry) => entry.type === "directory");
  entries.sort((left, right) => right.path.length - left.path.length);
  for (const entry of entries) {
    const rel = normalizeRelative(outDir, entry.path);
    if (rel === "retention/runtime-tree" || rel.endsWith("/retention/runtime-tree")) {
      fs.rmSync(entry.path, { recursive: true, force: true });
      removed += 1;
    }
  }
  return removed;
}

export function sanitizeOutputTree(outDir) {
  let sanitizedFiles = 0;
  for (const entry of walkTree(outDir)) {
    if (entry.type !== "file") continue;
    const buffer = fs.readFileSync(entry.path);
    if (!isText(buffer)) continue;
    const text = buffer.toString("utf8");
    const sanitized = sanitizeText(text);
    if (sanitized !== text) {
      fs.writeFileSync(entry.path, sanitized);
      sanitizedFiles += 1;
    }
  }
  return sanitizedFiles;
}

export function scanOutputTree(outDir) {
  const findings = [];
  for (const entry of walkTree(outDir)) {
    const rel = normalizeRelative(outDir, entry.path);
    if (forbiddenArtifact.test(rel)) {
      findings.push(`${entry.type}-path:${rel}`);
    }
    if (entry.type !== "file") continue;
    const buffer = fs.readFileSync(entry.path);
    if (forbiddenBytes.some((needle) => buffer.includes(Buffer.from(needle)))) {
      findings.push(`content:${rel}`);
    }
    if (isText(buffer) && forbiddenArtifact.test(buffer.toString("utf8"))) {
      findings.push(`content:${rel}`);
    }
  }
  return [...new Set(findings)].sort();
}
