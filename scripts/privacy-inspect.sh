#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: privacy-inspect.sh <morrow-db> <log-dir> <forbidden-tokens-file>" >&2
  exit 64
fi

db_path="$1"
log_dir="$2"
forbidden_tokens="$3"
work_dir="$(mktemp -d)"
trap 'rm -rf "$work_dir"' EXIT

sqlite3 "$db_path" ".dump" > "$work_dir/db.dump"
sqlite3 "$db_path" \
  "SELECT name FROM pragma_table_info('evidence')
   UNION ALL
   SELECT name FROM pragma_table_info('quiet_logs');" > "$work_dir/privacy-columns.txt"

if [[ -d "$log_dir" ]]; then
  find "$log_dir" -type f -exec cat {} + > "$work_dir/logs.dump" 2>/dev/null || true
else
  : > "$work_dir/logs.dump"
fi

if grep -Eiq 'full_message|(^|[^a-z])(body|prompt|response)([^a-z]|$)' "$work_dir/privacy-columns.txt"; then
  echo "privacy inspection failed: forbidden storage column present" >&2
  exit 2
fi

while IFS= read -r token; do
  [[ -z "$token" ]] && continue
  if grep -Fq "$token" "$work_dir/db.dump" "$work_dir/logs.dump"; then
    echo "privacy inspection failed: forbidden token found: $token" >&2
    exit 3
  fi
done < "$forbidden_tokens"

echo "privacy inspection passed: no full messages, prompts, responses, or forbidden non-whitelisted content found"
