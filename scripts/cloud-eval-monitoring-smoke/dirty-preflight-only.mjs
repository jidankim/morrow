#!/usr/bin/env node
import fs from "node:fs";

const report = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const dirty = report.dirty_worktree_summary?.entries || [];
const unexpected = report.dirty_worktree_summary?.unexpected_entries || dirty.filter((entry) => entry.allowed === false);
const unexpectedDirtyOnly = report.status === "FAIL"
  && report.mode === "fixture_smoke"
  && report.privacy?.status === "PASS"
  && (report.malformed_receipts || []).length === 0
  && report.contradiction_fixture === null
  && unexpected.length > 0;

if (unexpectedDirtyOnly) {
  console.error(JSON.stringify({
    status: "UNEXPECTED_DIRTY_BLOCKED",
    preflight_status: report.status,
    mode: report.mode,
    unexpected_entry_count: unexpected.length,
    unexpected_entries: unexpected.map((entry) => entry.path),
  }, null, 2));
}

process.exit(unexpectedDirtyOnly ? 0 : 1);
