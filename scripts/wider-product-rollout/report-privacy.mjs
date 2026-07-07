import { readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";
import { buildInvocation, writeReport } from "./report-io.mjs";

const FORBIDDEN_PATTERNS = [
  ["privacy_canary_literal", /MORROW_PRIVACY_CANARY_RAW|PHASE6_RAW_CONTENT_CANARY_DO_NOT_STORE/],
  ["private_path", /(?:\/Users\/|\/private\/|\/var\/folders\/|~\/\.codex)/],
  ["credential_boundary", /(?:codex_access_token|CODEX_ACCESS_TOKEN|auth\.json|provider token|sk-[A-Za-z0-9]{12,})/],
  ["raw_private_content", /(?:raw message|raw calendar|raw provider|actual user message|real user message)/i],
  ["native_identifier", /\b(?:message_guid|chat_identifier|ROWID|Z_PK|EventKit identifier)\b/i],
];

function safeRel(filePath) {
  const relative = path.relative(process.cwd(), filePath);
  return relative.startsWith("..") ? path.basename(filePath) : relative;
}

async function listFiles(target) {
  const info = await stat(target);
  if (info.isFile()) return [target];
  if (!info.isDirectory()) return [];
  const entries = await readdir(target, { withFileTypes: true });
  const nested = await Promise.all(entries.map((entry) => listFiles(path.join(target, entry.name))));
  return nested.flat();
}

export async function runPrivacyScan(args) {
  if (!args.path || !args.out_dir) {
    throw new Error("Usage: privacy-scan --path <file-or-dir> --out-dir <dir>");
  }

  const files = await listFiles(args.path);
  const findings = [];
  for (const file of files) {
    const rel = safeRel(file);
    for (const [name, pattern] of FORBIDDEN_PATTERNS) {
      if (pattern.test(rel)) findings.push({ file: rel, class: name, surface: "path" });
    }
    const text = await readFile(file, "utf8");
    for (const [name, pattern] of FORBIDDEN_PATTERNS) {
      if (pattern.test(text)) findings.push({ file: rel, class: name, surface: "content" });
    }
  }

  const status = findings.length === 0 ? "PASS" : "FAIL";
  const report = {
    mode: "privacy-scan",
    status,
    validated_at: new Date().toISOString(),
    invocation: buildInvocation(),
    scanned_path: safeRel(args.path),
    scanned_file_count: files.length,
    rejected_canary_or_private_content: status === "FAIL",
    findings,
  };
  const reportPath = await writeReport(args.out_dir, "privacy-scan.json", report);
  console.log(`${status} privacy scan: ${reportPath}`);
  if (status !== "PASS") process.exitCode = 1;
}
