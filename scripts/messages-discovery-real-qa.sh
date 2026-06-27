#!/usr/bin/env bash
set -euo pipefail

messages_db="${MORROW_MESSAGES_DB:-$HOME/Library/Messages/chat.db}"
qa_db="${MORROW_MESSAGES_QA_DB:-/tmp/morrow-native-messages-discovery.sqlite}"
tmp_dir="$(mktemp -d)"
last_sqlite_error="$tmp_dir/sqlite-error.txt"
trap 'rm -rf "$tmp_dir"' EXIT

sqlite_args=(
  -batch
  -noheader
  -readonly
  -separator $'\037'
)

startup_sql=$'PRAGMA query_only = ON;\nPRAGMA trusted_schema = OFF;\n'

sql_literal() {
  local value="$1"
  printf "'%s'" "${value//\'/\'\'}"
}

init_qa_db() {
  sqlite3 "$qa_db" <<'SQL'
DROP TABLE IF EXISTS evidence;
DROP TABLE IF EXISTS quiet_logs;
CREATE TABLE evidence (
  field_key TEXT NOT NULL,
  field_value TEXT NOT NULL
);
CREATE TABLE quiet_logs (
  event_key TEXT NOT NULL,
  event_value TEXT NOT NULL
);
SQL
}

record() {
  local key="$1"
  local value="$2"
  printf '%s=%s\n' "$key" "$value"
  sqlite3 "$qa_db" "INSERT INTO evidence (field_key, field_value) VALUES ($(sql_literal "$key"), $(sql_literal "$value"));"
}

record_quiet() {
  local key="$1"
  local value="$2"
  sqlite3 "$qa_db" "INSERT INTO quiet_logs (event_key, event_value) VALUES ($(sql_literal "$key"), $(sql_literal "$value"));"
}

run_messages_sql() {
  local sql="$1"
  local stderr_file="$tmp_dir/sqlite-stderr.txt"
  local output
  local status

  set +e
  output="$(sqlite3 "${sqlite_args[@]}" "$messages_db" 2>"$stderr_file" <<SQL
$startup_sql$sql
SQL
)"
  status=$?
  set -e

  if [ "$status" -ne 0 ]; then
    cp "$stderr_file" "$last_sqlite_error"
    return "$status"
  fi

  printf '%s' "$output"
}

sqlite_error_is_permission_denied() {
  local lower
  lower="$(tr '[:upper:]' '[:lower:]' < "$last_sqlite_error")"
  [[ "$lower" == *"permission denied"* ]] ||
    [[ "$lower" == *"operation not permitted"* ]] ||
    [[ "$lower" == *"authorization denied"* ]] ||
    [[ "$lower" == *"not authorized"* ]] ||
    [[ "$lower" == *"unable to open database file"* ]]
}

finish_blocked() {
  record "schema_compatible" "unknown"
  record "discovery_query_can_run" "false"
  record "message_body_dumped" "false"
  record "raw_handles_dumped" "false"
  record "chat_transcripts_dumped" "false"
  record_quiet "outcome" "blocked_full_disk_access_required"
  printf 'BLOCKED: Full Disk Access required\n'
}

finish_unavailable() {
  local reason="$1"
  record "schema_compatible" "false"
  record "discovery_query_can_run" "false"
  record "message_body_dumped" "false"
  record "raw_handles_dumped" "false"
  record "chat_transcripts_dumped" "false"
  record_quiet "outcome" "unavailable"
  printf 'FAIL messages_discovery_real_qa: %s\n' "$reason" >&2
  exit 2
}

schema_sql="
WITH required(required_table, required_column) AS (
  VALUES
    ('chat', 'guid'),
    ('chat', 'display_name'),
    ('message', 'guid'),
    ('message', 'date'),
    ('message', 'text'),
    ('chat_message_join', 'chat_id'),
    ('chat_message_join', 'message_id'),
    ('chat_handle_join', 'chat_id'),
    ('chat_handle_join', 'handle_id')
),
present(present_table, present_column) AS (
  SELECT 'chat', name FROM pragma_table_info('chat')
  UNION ALL SELECT 'message', name FROM pragma_table_info('message')
  UNION ALL SELECT 'chat_message_join', name FROM pragma_table_info('chat_message_join')
  UNION ALL SELECT 'chat_handle_join', name FROM pragma_table_info('chat_handle_join')
)
SELECT required_table || '.' || required_column
FROM required
LEFT JOIN present
  ON present_table = required_table
 AND present_column = required_column
WHERE present_column IS NULL
ORDER BY required_table, required_column;
"

counts_sql="
SELECT 'chat_rows=' || COUNT(*) FROM chat;
SELECT 'message_rows=' || COUNT(*) FROM message;
SELECT 'chat_message_join_rows=' || COUNT(*) FROM chat_message_join;
SELECT 'chat_handle_join_rows=' || COUNT(*) FROM chat_handle_join;
"

timestamp_sql="
SELECT CASE
  WHEN COUNT(*) = 0 THEN 'none'
  WHEN MAX(date) >= 10000000000 THEN 'apple_nanoseconds_since_2001'
  ELSE 'apple_seconds_since_2001'
END
FROM message
WHERE date IS NOT NULL;
"

discovery_sql="
WITH ordered_participants AS (
  SELECT DISTINCT chat_id, handle_id
  FROM chat_handle_join
  ORDER BY chat_id, handle_id
),
participants AS (
  SELECT chat_id, COUNT(*) AS participant_count
  FROM ordered_participants
  GROUP BY chat_id
),
chat_meta AS (
  SELECT c.ROWID AS chat_rowid, c.guid, MAX(m.date) AS latest_date
  FROM chat c
  JOIN chat_message_join cmj ON cmj.chat_id = c.ROWID
  JOIN message m ON m.ROWID = cmj.message_id
  GROUP BY c.ROWID, c.guid
)
SELECT COUNT(*)
FROM (
  SELECT cm.guid
  FROM chat_meta cm
  JOIN participants p ON p.chat_id = cm.chat_rowid
  ORDER BY cm.latest_date DESC, cm.guid ASC
  LIMIT 50
);
"

init_qa_db

printf 'messages_discovery_real_qa\n'
record "database_source" "$([ "${MORROW_MESSAGES_DB:-}" = "" ] && printf 'default_messages_chat_db' || printf 'custom_override')"
record "sqlite_readonly" "true"
record "sqlite_query_only" "true"
record "sqlite_trusted_schema" "false"
record "qa_db" "$qa_db"

if ! missing_columns="$(run_messages_sql "$schema_sql")"; then
  if sqlite_error_is_permission_denied; then
    finish_blocked
    exit 0
  fi
  finish_unavailable "Messages schema could not be inspected"
fi

if [ -n "$missing_columns" ]; then
  record "schema_compatible" "false"
  record "missing_schema_items" "$(printf '%s' "$missing_columns" | tr '\n' ',' | sed 's/,$//')"
  record "discovery_query_can_run" "false"
  record "message_body_dumped" "false"
  record "raw_handles_dumped" "false"
  record "chat_transcripts_dumped" "false"
  record_quiet "outcome" "schema_incompatible"
  printf 'FAIL messages_discovery_real_qa: Messages schema incompatible\n' >&2
  exit 2
fi

record "schema_compatible" "true"

if ! counts_output="$(run_messages_sql "$counts_sql")"; then
  if sqlite_error_is_permission_denied; then
    finish_blocked
    exit 0
  fi
  finish_unavailable "Messages row counts could not be read"
fi

while IFS= read -r count_line; do
  [ -z "$count_line" ] && continue
  record "${count_line%%=*}" "${count_line#*=}"
done <<< "$counts_output"

if ! timestamp_mode="$(run_messages_sql "$timestamp_sql")"; then
  if sqlite_error_is_permission_denied; then
    finish_blocked
    exit 0
  fi
  finish_unavailable "Messages timestamp mode could not be determined"
fi
record "timestamp_conversion_mode" "$timestamp_mode"

if ! discovered_chat_count="$(run_messages_sql "$discovery_sql")"; then
  if sqlite_error_is_permission_denied; then
    finish_blocked
    exit 0
  fi
  finish_unavailable "Messages discovery query could not run"
fi

record "discovery_query_can_run" "true"
record "discovered_chat_count" "$discovered_chat_count"
record "message_body_dumped" "false"
record "raw_handles_dumped" "false"
record "chat_transcripts_dumped" "false"
record_quiet "outcome" "pass"
printf 'PASS messages_discovery_real_qa\n'
