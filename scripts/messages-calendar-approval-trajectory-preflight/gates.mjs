import path from "node:path";

import { readJson } from "./io.mjs";

export function providerResolution(docs) {
  const hasHistoricalConflict =
    docs.current_readme_codex_cli &&
    docs.current_readme_api_key_not_required &&
    docs.old_plan_openai_api_key_required;
  return {
    status: hasHistoricalConflict ? "RECONCILED" : "NO_CONFLICT_DETECTED",
    current_source_of_truth: docs.current_readme_codex_cli
      ? "README Codex CLI session-backed provider path"
      : "UNKNOWN",
    historical_conflict: hasHistoricalConflict
      ? "older production-wiring plan and task-7 receipt used an OpenAI API-key/provider-token gate"
      : null,
    live_gate_provider_auth: docs.live_script_codex_cli_session
      ? "Codex CLI session"
      : docs.live_script_requires_openai_api_key
        ? "OpenAI API key"
        : "UNKNOWN",
    explicit_reconciliation_note:
      "Phase 5 live work must default to the current README/Codex CLI provider-auth path. The older API-key production-wiring plan is retained as historical predecessor evidence and cannot make the current live gate require MORROW_REAL_QA_OPENAI_API_KEY without an explicit source-of-truth update.",
    readme_says_codex_cli: docs.current_readme_codex_cli,
    readme_says_api_key_not_required: docs.current_readme_api_key_not_required,
    old_plan_requires_api_key: docs.old_plan_openai_api_key_required,
    live_command_requires_api_key: docs.live_script_requires_openai_api_key,
  };
}

export function validateProviderConflictFixture(fixturePath) {
  if (!fixturePath) {
    return null;
  }
  const fixture = readJson(path.resolve(fixturePath));
  const readmeSaysCodex =
    fixture.readme_says_codex_cli === true ||
    /codex cli|codex login/i.test(String(fixture.readme_provider_auth ?? ""));
  const liveRequiresApiKey =
    fixture.live_command_requires_api_key === true ||
    /MORROW_REAL_QA_OPENAI_API_KEY/.test(String(fixture.live_command ?? ""));
  const note = String(fixture.reconciliation_note ?? "").trim();
  const noteIsExplicit =
    note.length > 0 &&
    /codex cli/i.test(note) &&
    /MORROW_REAL_QA_OPENAI_API_KEY|api[- ]?key|historical/i.test(note);

  return {
    fixture_path: fixturePath,
    readme_says_codex_cli: readmeSaysCodex,
    live_command_requires_api_key: liveRequiresApiKey,
    explicit_reconciliation_note_present: noteIsExplicit,
    status:
      readmeSaysCodex && liveRequiresApiKey && !noteIsExplicit
        ? "PROVIDER_AUTH_CONTRADICTION"
        : "PASS",
  };
}

export function dirtyWorktreeAllowlist(gitStatus) {
  const allowedPrefixes = [
    ".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/preflight",
    ".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/preflight-negative",
    "scripts/messages-calendar-approval-trajectory-preflight/",
  ];
  const allowedExact = new Set([
    "scripts/messages-calendar-approval-trajectory-preflight.mjs",
  ]);
  const entries = gitStatus
    .split("\n")
    .filter((line) => line.trim().length > 0)
    .map((line) => {
      const status = line.slice(0, 2);
      const filePath = line.slice(3);
      const allowed =
        allowedExact.has(filePath) ||
        allowedPrefixes.some((prefix) => filePath.startsWith(prefix));
      return { status, path: filePath, allowed };
    });
  return {
    policy:
      "Allowed dirty entries are this Todo 1 preflight script/modules and generated Phase 5 preflight evidence only.",
    entries,
    unexpected_entries: entries.filter((entry) => !entry.allowed),
    all_entries_allowed: entries.every((entry) => entry.allowed),
  };
}

export function liveGateClassification({ messagesCalendarTask7, providerResolution, dirtyAllowlist }) {
  const livePassExists = messagesCalendarTask7.status === "PASS";
  const currentProviderGateOk =
    providerResolution.status === "RECONCILED" ||
    providerResolution.status === "NO_CONFLICT_DETECTED";
  const passCapable = currentProviderGateOk && dirtyAllowlist.all_entries_allowed;
  const blockers = [];
  if (!livePassExists) {
    blockers.push("no live Messages-to-Calendar PASS receipt found");
  }
  if (!dirtyAllowlist.all_entries_allowed) {
    blockers.push("unexpected dirty worktree entries outside preflight allowlist");
  }
  if (!currentProviderGateOk) {
    blockers.push("provider-auth source of truth is not reconciled");
  }
  if (messagesCalendarTask7.blocked_reason) {
    blockers.push(`historical task-7 blocker: ${messagesCalendarTask7.blocked_reason}`);
  }
  return {
    status: livePassExists ? "PASS" : "BLOCKED",
    pass_capable_after_prerequisites: passCapable,
    live_claim_allowed: livePassExists,
    blockers,
  };
}

export function overallStatus({ fixtureFailed, dirtyAllowlist }) {
  if (fixtureFailed) {
    return "FAIL";
  }
  return dirtyAllowlist.all_entries_allowed ? "PASS" : "BLOCKED";
}
