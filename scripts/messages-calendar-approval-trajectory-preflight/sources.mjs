import path from "node:path";

import { exists, readText, runGit } from "./io.mjs";

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

export function discoverHistoricalRoot(repoRoot) {
  const expectedFiles = [
    ".omo/drafts/messages-calendar-approval-trajectory-eval.md",
    ".omo/plans/messages-calendar-production-wiring.md",
    ".omo/evidence/phase-4-human-approval-correction/final-smoke/summary.txt",
    ".omo/evidence/messages-calendar-production-wiring/task-7/real-qa-run.log",
  ];
  const candidates = [{ root: repoRoot, method: "current_worktree" }];
  const worktreeOutput = runGit(["worktree", "list", "--porcelain"]);
  for (const worktree of parseWorktreeList(worktreeOutput)) {
    candidates.push({ root: worktree.path, method: "git_worktree_list" });
  }

  for (const candidate of candidates) {
    const foundCount = expectedFiles.filter((relativePath) =>
      exists(path.join(candidate.root, relativePath)),
    ).length;
    if (foundCount > 0) {
      return {
        root: candidate.root,
        method: candidate.method,
        found_expected_file_count: foundCount,
      };
    }
  }
  return {
    root: repoRoot,
    method: "current_worktree_fallback",
    found_expected_file_count: 0,
  };
}

export function sourcePath(rootInfo, relativePath) {
  return path.join(rootInfo.root, relativePath);
}

export function sourceLabel(rootInfo, repoRoot) {
  return rootInfo.root === repoRoot ? "current_omo" : "historical_omo";
}

export function providerAuthDocs(repoRoot, historicalRootInfo) {
  const readme = readText(path.join(repoRoot, "README.md"));
  const productionPlanPath = sourcePath(
    historicalRootInfo,
    ".omo/plans/messages-calendar-production-wiring.md",
  );
  const productionPlan = exists(productionPlanPath) ? readText(productionPlanPath) : "";
  const realQaScriptPath = path.join(repoRoot, "scripts/messages-calendar-real-qa.sh");
  const realQaScript = exists(realQaScriptPath) ? readText(realQaScriptPath) : "";
  const realQaRuntimePath = path.join(repoRoot, "scripts/messages-calendar-real-qa/evidence.sh");
  const realQaRuntime = exists(realQaRuntimePath) ? readText(realQaRuntimePath) : "";

  return {
    current_readme_codex_cli: /\bcodex login\b/i.test(readme) && /\bCodex CLI\b/i.test(readme),
    current_readme_api_key_not_required: /MORROW_REAL_QA_OPENAI_API_KEY`? is not required/i.test(
      readme,
    ),
    old_plan_openai_api_key_required: /MORROW_REAL_QA_OPENAI_API_KEY/.test(productionPlan),
    live_script_requires_openai_api_key:
      /MORROW_REAL_QA_OPENAI_API_KEY/.test(realQaScript) ||
      /MORROW_REAL_QA_OPENAI_API_KEY/.test(realQaRuntime),
    live_script_codex_cli_session: /codex_cli_session/.test(realQaScript + "\n" + realQaRuntime),
    sources: [
      { root: "current_worktree", path: "README.md" },
      {
        root: sourceLabel(historicalRootInfo, repoRoot),
        path: ".omo/plans/messages-calendar-production-wiring.md",
      },
      { root: "current_worktree", path: "scripts/messages-calendar-real-qa.sh" },
      { root: "current_worktree", path: "scripts/messages-calendar-real-qa/evidence.sh" },
    ],
  };
}
