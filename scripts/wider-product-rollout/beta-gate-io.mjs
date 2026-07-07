import { mkdir, readFile, realpath, rm, stat } from "node:fs/promises";
import path from "node:path";

export function parseArgs(args) {
  const parsed = {};
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--bundle-dir" || arg === "--out-dir") {
      const value = args[index + 1];
      if (!value || value.startsWith("--")) {
        throw new Error(`${arg} requires a value`);
      }
      parsed[arg.slice(2).replace("-", "_")] = value;
      index += 1;
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  if (!parsed.bundle_dir || !parsed.out_dir) {
    throw new Error("Usage: node scripts/wider-product-rollout-beta-gate.mjs --bundle-dir <bundle-dir> --out-dir <out-dir>");
  }
  return { bundleDir: parsed.bundle_dir, outDir: parsed.out_dir };
}

export async function exists(filePath) {
  try {
    return await stat(filePath);
  } catch {
    return undefined;
  }
}

export async function readIfPresent(filePath) {
  try {
    return await readFile(filePath, "utf8");
  } catch {
    return "";
  }
}

function isSameOrParent(parentPath, childPath) {
  const relative = path.relative(parentPath, childPath);
  return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative));
}

async function canonicalPath(repoRoot, filePath) {
  const absolutePath = path.resolve(repoRoot, filePath);
  const missingParts = [];
  let existingPath = absolutePath;

  while (!(await exists(existingPath))) {
    const parentPath = path.dirname(existingPath);
    if (parentPath === existingPath) {
      break;
    }
    missingParts.unshift(path.basename(existingPath));
    existingPath = parentPath;
  }

  const canonicalExistingPath = await realpath(existingPath);
  return path.resolve(canonicalExistingPath, ...missingParts);
}

export async function resetOutputDirectory(repoRoot, outDir, bundleDir) {
  const canonicalOutDir = await canonicalPath(repoRoot, outDir);
  const canonicalRepoRoot = await canonicalPath(repoRoot, repoRoot);
  const canonicalRepoParent = await canonicalPath(repoRoot, path.dirname(repoRoot));
  const canonicalBundleDir = await canonicalPath(repoRoot, bundleDir);
  const canonicalRootPath = await canonicalPath(repoRoot, path.parse(canonicalOutDir).root);
  if (
    canonicalOutDir === canonicalRootPath ||
    canonicalOutDir === canonicalRepoRoot ||
    canonicalOutDir === canonicalRepoParent ||
    isSameOrParent(canonicalOutDir, canonicalRepoRoot) ||
    isSameOrParent(canonicalOutDir, canonicalBundleDir)
  ) {
    throw new Error(`refusing to clear unsafe output directory: ${outDir}`);
  }
  await rm(outDir, { recursive: true, force: true });
  await mkdir(outDir, { recursive: true });
}
