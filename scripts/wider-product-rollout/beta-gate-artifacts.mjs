import { readdir } from "node:fs/promises";
import path from "node:path";

import { exists } from "./beta-gate-io.mjs";

export async function discoverArtifacts(bundleDir) {
  const result = { apps: [], dmgs: [], diagnostic: [] };
  async function walk(dir) {
    let entries = [];
    try {
      entries = await readdir(dir, { withFileTypes: true });
    } catch {
      return;
    }
    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);
      const lower = entry.name.toLowerCase();
      if (entry.isDirectory() && entry.name === "Morrow.app") {
        result.apps.push(fullPath);
      } else if (entry.isFile() && lower.endsWith(".dmg")) {
        result.dmgs.push(fullPath);
      } else if (entry.isFile() && (lower.includes("diagnostic") || lower.endsWith(".zip"))) {
        result.diagnostic.push(fullPath);
      } else if (entry.isDirectory() && !entry.name.endsWith(".app")) {
        await walk(fullPath);
      }
    }
  }
  if ((await exists(bundleDir))?.isDirectory()) {
    await walk(bundleDir);
  }
  return result;
}
