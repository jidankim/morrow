import path from "node:path";

import { exists, readText, runGit, sourceReceipt } from "./io.mjs";

export function parseWorktreeList(output) {
  const worktrees = [];
  let current = null;
  for (const line of output.split("\n")) {
    if (line.startsWith("worktree ")) {
      if (current) {
        worktrees.push(current);
      }
      current = { path: line.slice("worktree ".length), attributes: [] };
    } else if (current && line.trim().length > 0) {
      current.attributes.push(line);
    }
  }
  if (current) {
    worktrees.push(current);
  }
  return worktrees;
}

export function discoverReceiptRoot(repoRoot) {
  const expectedFiles = [
    ".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke/summary.txt",
    ".omo/evidence/phase-4-human-approval-correction/final-smoke/summary.txt",
    ".omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/summary.txt",
    ".omo/evidence/phase-2-trace-candidate-correlation/final-smoke/summary.txt",
  ];
  const candidates = [{ root: repoRoot, method: "current_worktree" }];
  for (const worktree of parseWorktreeList(runGit(["worktree", "list", "--porcelain"]))) {
    candidates.push({ root: worktree.path, method: "git_worktree_list" });
  }
  let best = { root: repoRoot, method: "current_worktree_fallback", found_expected_file_count: 0 };
  for (const candidate of candidates) {
    const foundCount = expectedFiles.filter((relativePath) =>
      exists(path.join(candidate.root, relativePath)),
    ).length;
    if (foundCount > best.found_expected_file_count) {
      best = { ...candidate, found_expected_file_count: foundCount };
    }
  }
  return best;
}

export function rootLabel(rootInfo, repoRoot) {
  return rootInfo.root === repoRoot ? "current_omo" : "historical_omo";
}

export function sourcePath(rootInfo, relativePath) {
  return path.join(rootInfo.root, relativePath);
}

export function currentDocs(repoRoot) {
  const files = [
    "README.md",
    "docs/axi-phase-0-feedback-eval-baseline.md",
    "docs/diagnostics-trace-eval.md",
    "docs/architecture/sans-io-boundaries.md",
    ".omo/plans/cloud-eval-monitoring.md",
    ".omo/drafts/cloud-eval-monitoring.md",
  ];
  const textByPath = {};
  for (const relativePath of files) {
    const absolutePath = path.join(repoRoot, relativePath);
    textByPath[relativePath] = exists(absolutePath) ? readText(absolutePath) : "";
  }
  return {
    cloud_telemetry_future_work: /cloud telemetry remains future work/i.test(
      `${textByPath["README.md"]}\n${textByPath["docs/axi-phase-0-feedback-eval-baseline.md"]}`,
    ),
    phase6_plan_present: textByPath[".omo/plans/cloud-eval-monitoring.md"].length > 0,
    docs_read: files.map((relativePath) => ({
      path: relativePath,
      present: textByPath[relativePath].length > 0,
    })),
  };
}

export function sourceReceipts(repoRoot, receiptRootInfo) {
  const label = rootLabel(receiptRootInfo, repoRoot);
  return [
    sourceReceipt("current_worktree", "README.md", path.join(repoRoot, "README.md")),
    sourceReceipt(
      "current_worktree",
      "docs/axi-phase-0-feedback-eval-baseline.md",
      path.join(repoRoot, "docs/axi-phase-0-feedback-eval-baseline.md"),
    ),
    sourceReceipt(
      "current_worktree",
      "docs/diagnostics-trace-eval.md",
      path.join(repoRoot, "docs/diagnostics-trace-eval.md"),
    ),
    sourceReceipt(
      "current_worktree",
      "docs/architecture/sans-io-boundaries.md",
      path.join(repoRoot, "docs/architecture/sans-io-boundaries.md"),
    ),
    sourceReceipt(
      "current_worktree",
      ".omo/plans/cloud-eval-monitoring.md",
      path.join(repoRoot, ".omo/plans/cloud-eval-monitoring.md"),
    ),
    sourceReceipt("current_worktree", "package.json", path.join(repoRoot, "package.json")),
    sourceReceipt(
      "current_worktree",
      "src-tauri/tauri.conf.json",
      path.join(repoRoot, "src-tauri/tauri.conf.json"),
    ),
    sourceReceipt(
      label,
      ".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke/summary.txt",
      sourcePath(
        receiptRootInfo,
        ".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke/summary.txt",
      ),
    ),
  ];
}
