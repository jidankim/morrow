import path from "node:path";

export const phaseRoot = ".omo/evidence/phase-6-cloud-eval-monitoring";
export const reportJsonName = "preflight-report.json";
export const reportMdName = "preflight-report.md";

const valueArgs = new Set(["--out-dir", "--fixture-contradiction", "--fixture-git-status"]);
const flagArgs = new Set(["--require-cloud-prerequisites"]);

export function parseArgs(argv, repoRoot) {
  const parsed = new Map();
  const flags = new Set();
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (!arg.startsWith("--")) {
      throw new Error(`unexpected positional argument: ${arg}`);
    }
    if (flagArgs.has(arg)) {
      flags.add(arg);
      continue;
    }
    if (!valueArgs.has(arg)) {
      throw new Error(`unknown argument: ${arg}`);
    }
    const value = argv[index + 1];
    if (!value || value.startsWith("--")) {
      throw new Error(`missing value for ${arg}`);
    }
    parsed.set(arg, value);
    index += 1;
  }
  const outDir = parsed.get("--out-dir");
  if (!outDir) {
    throw new Error("required argument missing: --out-dir");
  }
  return {
    outDir,
    outDirAbs: scopedOutDir(repoRoot, outDir),
    fixtureContradiction: parsed.get("--fixture-contradiction") ?? null,
    fixtureGitStatus: parsed.get("--fixture-git-status") ?? null,
    requireCloudPrerequisites: flags.has("--require-cloud-prerequisites"),
  };
}

export function scopedOutDir(repoRoot, outDir) {
  const phaseRootAbs = path.resolve(repoRoot, phaseRoot);
  const resolved = path.resolve(repoRoot, outDir);
  const relative = path.relative(phaseRootAbs, resolved);
  if (relative === "" || relative.startsWith("..") || path.isAbsolute(relative)) {
    throw new Error(`--out-dir must be under ${phaseRoot}; received ${outDir}`);
  }
  return resolved;
}
