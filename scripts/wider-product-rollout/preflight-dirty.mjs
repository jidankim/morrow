const OWNED_SOURCE_PATHS = new Set([
  "scripts/wider-product-rollout-preflight.mjs",
  "scripts/wider-product-rollout-fixtures/preflight-phase6-deployed-contradiction.json"
]);

function isOwnedPreflightHelper(filePath) {
  return filePath.startsWith("scripts/wider-product-rollout/preflight-") && filePath.endsWith(".mjs");
}

function parseGitStatus(statusText) {
  if (statusText.trim() === "") {
    return [];
  }
  return statusText
    .split("\n")
    .filter((line) => line.length > 0)
    .map((line) => {
      const status = line.slice(0, 2);
      const rawPath = line.slice(3);
      const filePath = rawPath.includes(" -> ") ? rawPath.split(" -> ").at(-1) : rawPath;
      return { status, path: filePath };
    });
}

function classifyPath(entry) {
  const filePath = entry.path;
  if (OWNED_SOURCE_PATHS.has(filePath) || isOwnedPreflightHelper(filePath)) {
    return {
      ...entry,
      classification: "owned_todo1_source",
      boundary: "product_tooling",
      allowed_for_task: true
    };
  }
  if (filePath.startsWith(".omo/evidence/phase-7-wider-product-rollout/task-1/")) {
    return {
      ...entry,
      classification: "owned_todo1_evidence",
      boundary: "planning_evidence",
      allowed_for_task: true
    };
  }
  if (filePath.startsWith("docs/plans/beta-release-distribution/")) {
    return {
      ...entry,
      classification: "docs_mirror_noncanonical",
      boundary: "docs_mirror",
      allowed_for_task: false,
      source_of_truth: ".omo/plans/beta-release-distribution.md"
    };
  }
  if (filePath === "public/assets/morrow-hero.png") {
    return {
      ...entry,
      classification: "unrelated_hero_asset",
      boundary: "unrelated_asset",
      allowed_for_task: false,
      phase7_intentional_use: false
    };
  }
  if (filePath.startsWith(".omo/evidence/")) {
    return {
      ...entry,
      classification: "planning_evidence_artifact",
      boundary: "planning_evidence",
      allowed_for_task: false
    };
  }
  if (
    filePath.startsWith("src/") ||
    filePath.startsWith("src-tauri/") ||
    filePath.startsWith("crates/") ||
    filePath === "package.json"
  ) {
    return {
      ...entry,
      classification: "product_runtime_or_build_artifact",
      boundary: "product_runtime",
      allowed_for_task: false
    };
  }
  if (filePath.startsWith("docs/") || filePath === "README.md") {
    return {
      ...entry,
      classification: "docs_artifact",
      boundary: "docs",
      allowed_for_task: false
    };
  }
  return {
    ...entry,
    classification: "other_worker_or_unrelated",
    boundary: "unknown",
    allowed_for_task: false
  };
}

export function buildDirtySummary(statusText) {
  const entries = parseGitStatus(statusText).map(classifyPath);
  return {
    policy:
      "Do not fail only because the shared worktree is dirty; snapshot and classify every path before later rollout work depends on it.",
    status: entries.length === 0 ? "CLEAN" : "DIRTY_RECORDED",
    entry_count: entries.length,
    entries,
    docs_mirror_paths: entries.filter((entry) => entry.classification === "docs_mirror_noncanonical"),
    hero_asset_paths: entries.filter((entry) => entry.path === "public/assets/morrow-hero.png"),
    product_runtime_entries: entries.filter((entry) => entry.boundary === "product_runtime"),
    planning_evidence_entries: entries.filter((entry) => entry.boundary === "planning_evidence")
  };
}
