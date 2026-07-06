import path from "node:path";

export const phaseRoot =
  ".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval";
export const reportJsonName = "preflight-report.json";
export const reportMdName = "preflight-report.md";

const allowedArgs = new Set([
  "--out-dir",
  "--fixture-provider-conflict",
  "--fixture-git-status",
]);

export function parseArgs(argv, repoRoot) {
  const parsed = new Map();
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (!arg.startsWith("--")) {
      throw new Error(`unexpected positional argument: ${arg}`);
    }
    if (!allowedArgs.has(arg)) {
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
    outDirAbs: scopedPreflightOutDir(repoRoot, outDir),
    fixtureProviderConflict: parsed.get("--fixture-provider-conflict") ?? null,
    fixtureGitStatus: parsed.get("--fixture-git-status") ?? null,
  };
}

export function scopedPreflightOutDir(repoRoot, outDir) {
  const phaseRootAbs = path.resolve(repoRoot, phaseRoot);
  const resolved = path.resolve(repoRoot, outDir);
  const relative = path.relative(phaseRootAbs, resolved);
  if (relative === "" || relative.startsWith("..") || path.isAbsolute(relative)) {
    throw new Error(
      `--out-dir must be under ${phaseRoot}/preflight*; received ${outDir}`,
    );
  }
  const firstSegment = relative.split(path.sep)[0] ?? "";
  if (!firstSegment.startsWith("preflight")) {
    throw new Error(
      `--out-dir must be scoped to ${phaseRoot}/preflight*; received ${outDir}`,
    );
  }
  return resolved;
}
