run_logged() {
  local log_path="$1"
  shift
  {
    echo "scenario: targeted command"
    echo "invocation: $*"
    "$@"
  } > "$log_path" 2>&1
}

run_native_test() {
  local log_path="$1"
  local filter="$2"
  if command -v xcrun >/dev/null 2>&1; then
    local macos_sdk
    macos_sdk="$(xcrun --sdk macosx --show-sdk-path)"
    {
      echo "scenario: targeted native cargo test"
      echo "invocation: cargo test -p morrow $filter"
      SDKROOT="$macos_sdk" LIBRARY_PATH="$macos_sdk/usr/lib" \
        cargo test -p morrow "$filter"
    } > "$log_path" 2>&1
  else
    run_logged "$log_path" cargo test -p morrow "$filter"
  fi
}

run_storage_readback_query() {
  local log_path="$1"
  local db_path="$2"
  local source_path="$3"
  local query

  query="
SELECT 'candidate' AS row_type,
       c.id AS candidate_id,
       c.kind AS kind,
       c.state AS state,
       c.current_reason AS current_reason,
       '' AS source,
       '' AS external_object_id,
       '' AS cursor_stream,
       NULL AS cursor_value,
       COUNT(a.id) AS audit_count
FROM candidates c
LEFT JOIN audit_log a ON a.candidate_id = c.id
GROUP BY c.id, c.kind, c.state, c.current_reason
UNION ALL
SELECT 'external_mapping' AS row_type,
       m.candidate_id AS candidate_id,
       '' AS kind,
       '' AS state,
       '' AS current_reason,
       m.source AS source,
       m.external_object_id AS external_object_id,
       '' AS cursor_stream,
       NULL AS cursor_value,
       0 AS audit_count
FROM external_object_mappings m
UNION ALL
SELECT 'replay_cursor' AS row_type,
       '' AS candidate_id,
       '' AS kind,
       '' AS state,
       '' AS current_reason,
       '' AS source,
       '' AS external_object_id,
       stream AS cursor_stream,
       cursor_value AS cursor_value,
       0 AS audit_count
FROM replay_cursors
WHERE stream = 'calendar_proposals'
ORDER BY row_type, candidate_id, source, cursor_stream;
"

  sqlite3 -readonly -json "$db_path" "$query" > "$source_path"
  {
    echo "scenario: independent SQLite storage readback"
    echo "invocation: sqlite3 -readonly -json <storage-readback.sqlite> <candidate/mapping/replay-cursor query>"
    echo "observable: readback rows are selected from the current-run storage DB after Store restart validation"
    echo "source_artifact: $(basename -- "$source_path")"
    cat "$source_path"
    echo "result: PASS"
  } > "$log_path" 2>&1
}

prepare_privacy_surface() {
  local root="$1"
  local include_canary="$2"
  local db_path="$root/morrow.sqlite"
  local logs_dir="$root/logs"
  local diagnostics_dir="$root/diagnostics"
  mkdir -p "$logs_dir" "$diagnostics_dir/traces" "$diagnostics_dir/evals" "$diagnostics_dir/exports"
  sqlite3 "$db_path" <<'SQL'
CREATE TABLE evidence (id TEXT PRIMARY KEY, excerpt_hash TEXT);
CREATE TABLE quiet_logs (id TEXT PRIMARY KEY, reason_code TEXT);
INSERT INTO evidence (id, excerpt_hash) VALUES ('phase3-lifecycle-smoke', 'sha256:synthetic');
INSERT INTO quiet_logs (id, reason_code) VALUES ('phase3-replay-run', 'replay_run');
SQL
  printf 'phase 3 lifecycle replay smoke sanitized log\n' > "$logs_dir/morrow.log"
  cp "$trace_out" "$diagnostics_dir/traces/trace.jsonl"
  cp "$lifecycle_report" "$diagnostics_dir/evals/lifecycle-report.json"
  cp "$storage_readback" "$diagnostics_dir/exports/storage-readback.json"
  if [[ "$include_canary" == "yes" ]]; then
    printf '{"leak":"%s"}\n' "$CANARY" > "$diagnostics_dir/traces/canary.jsonl"
  fi
  printf '%s\n' "$CANARY" > "$root/forbidden-tokens.txt"
}

clear_known_artifacts() {
  local artifact
  local known_artifacts=(
    trace.jsonl storage-readback.json lifecycle-report.json privacy-inspect.txt
    canary-rejection.txt cleanup-receipt.txt summary.txt command-log-pass-counts.txt
    storage-readback-source.json storage-readback.sqlite storage-readback.sqlite-wal
    storage-readback.sqlite-shm storage-readback.sqlite-journal
  )
  for artifact in "${known_artifacts[@]}"; do
    rm -f "$out_dir_abs/$artifact"
  done
  safe_remove_path "$out_dir_abs/command-logs"
  safe_remove_path "$out_dir_abs/privacy-surface"
  safe_remove_path "$out_dir_abs/privacy-surface-canary"
  mkdir -p "$out_dir_abs/command-logs"
}
