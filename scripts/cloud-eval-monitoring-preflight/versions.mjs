import path from "node:path";

import { exists, readJson } from "./io.mjs";

export function versionSources(repoRoot) {
  const packagePath = path.join(repoRoot, "package.json");
  const tauriPath = path.join(repoRoot, "src-tauri/tauri.conf.json");
  const packageJson = exists(packagePath) ? readJson(packagePath) : {};
  const tauriJson = exists(tauriPath) ? readJson(tauriPath) : {};
  return {
    package: {
      source: { root: "current_worktree", path: "package.json" },
      name: packageJson.name ?? null,
      version: packageJson.version ?? null,
    },
    app: {
      source: { root: "current_worktree", path: "src-tauri/tauri.conf.json" },
      product_name: tauriJson.productName ?? null,
      version: tauriJson.version ?? null,
      identifier_hash_present: Boolean(tauriJson.identifier),
    },
  };
}
