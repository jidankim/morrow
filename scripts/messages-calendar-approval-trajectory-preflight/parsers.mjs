import { exists, readJson, readText } from "./io.mjs";

export function firstMatch(text, pattern) {
  const match = text.match(pattern);
  return match ? match[1] : null;
}

export function statusFromText(text, patterns) {
  for (const [status, pattern] of patterns) {
    if (pattern.test(text)) {
      return status;
    }
  }
  return "UNKNOWN";
}

export function receiptSummary(rootLabel, relativePath, absolutePath, parser) {
  if (!exists(absolutePath)) {
    return {
      source: { root: rootLabel, path: relativePath },
      status: "MISSING",
      present: false,
    };
  }
  const text = readText(absolutePath);
  return {
    source: { root: rootLabel, path: relativePath },
    present: true,
    ...parser(text),
  };
}

export function parsePhaseSmoke(text) {
  return {
    status: statusFromText(text, [["PASS", /^result:\s*PASS$/m]]),
    invocation: firstMatch(text, /^invocation:\s*(.+)$/m),
    privacy_receipt_present: /privacy_(?:inspection|inspect):\s*\S+|privacy_inspection:\s*\S+/m.test(
      text,
    ),
    canary_rejection_present: /^canary_rejection:\s*\S+/m.test(text),
    cleanup_receipt_present: /^cleanup_receipt:\s*\S+/m.test(text),
  };
}

export function parseRealSurface(text) {
  return {
    status: statusFromText(text, [
      ["PASS", /^overall_status=PASS$/m],
      ["BLOCKED", /^overall_status=BLOCKED$/m],
      ["FAIL", /^overall_status=FAIL$/m],
    ]),
    invocation: firstMatch(text, /^invocation=(.+)$/m),
    calendar_status: firstMatch(text, /^calendar_status=(.+)$/m),
    reminders_status: firstMatch(text, /^reminders_status=(.+)$/m),
    cleanup_receipt_present: /^cleanup_receipt=.+$/m.test(text),
  };
}

export function parseMessagesCalendarTask7(text) {
  const outcome = firstMatch(text, /^outcome=(.+)$/m);
  return {
    status:
      outcome === "pass" || /^PASS\b/m.test(text)
        ? "PASS"
        : outcome === "blocked" || /^BLOCKED:/m.test(text)
          ? "BLOCKED"
          : outcome === "fail" || /^FAIL\b/m.test(text)
            ? "FAIL"
            : "UNKNOWN",
    outcome: outcome ?? "UNKNOWN",
    blocked_reason: firstMatch(text, /^blocked_reason=(.+)$/m),
    privacy_flags: {
      message_body_dumped: firstMatch(text, /^message_body_dumped=(.+)$/m),
      raw_handles_dumped: firstMatch(text, /^raw_handles_dumped=(.+)$/m),
      api_key_dumped: firstMatch(text, /^api_key_dumped=(.+)$/m),
      provider_payload_dumped: firstMatch(text, /^provider_payload_dumped=(.+)$/m),
    },
  };
}

export function checkboxState(planText, todoName = null) {
  const checkboxLines = planText
    .split("\n")
    .filter((line) => /^\s*- \[[ xX]\]/.test(line));
  const checked = checkboxLines.filter((line) => /^\s*- \[[xX]\]/.test(line)).length;
  const unchecked = checkboxLines.filter((line) => /^\s*- \[ \]/.test(line)).length;
  const matchingTodo = todoName
    ? checkboxLines.find((line) => line.toLowerCase().includes(todoName.toLowerCase())) ?? null
    : null;
  return { checked, unchecked, total: checkboxLines.length, matching_todo: matchingTodo };
}

export function planState(rootLabel, relativePath, absolutePath, todoName = null) {
  if (!exists(absolutePath)) {
    return {
      source: { root: rootLabel, path: relativePath },
      present: false,
      checkbox_state: null,
    };
  }
  return {
    source: { root: rootLabel, path: relativePath },
    present: true,
    checkbox_state: checkboxState(readText(absolutePath), todoName),
  };
}

export function boulderSnapshot(rootLabel, relativePath, absolutePath) {
  if (!exists(absolutePath)) {
    return { source: { root: rootLabel, path: relativePath }, present: false };
  }
  const boulder = readJson(absolutePath);
  const interesting = {};
  for (const workId of [
    "messages-calendar-approval-trajectory-eval",
    "messages-calendar-production-wiring",
    "morrow-trace-eval-layers",
    "lifecycle-replay-coverage",
    "sans-io-boundaries",
  ]) {
    const work = boulder.works?.[workId];
    if (work) {
      interesting[workId] = {
        status: work.status ?? null,
        active_plan: work.active_plan ?? null,
        worktree_path_present: Boolean(work.worktree_path),
      };
    }
  }
  return {
    source: { root: rootLabel, path: relativePath },
    present: true,
    schema_version: boulder.schema_version ?? null,
    active_work_id: boulder.active_work_id ?? null,
    works: interesting,
  };
}
