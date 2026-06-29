#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 && $# -ne 4 ]]; then
  echo "usage: privacy-inspect.sh <morrow-db> <log-dir> <forbidden-tokens-file> [diagnostics-dir]" >&2
  exit 64
fi

db_path="$1"
log_dir="$2"
forbidden_tokens="$3"
diagnostics_dir="${4:-}"
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

if [[ -n "$diagnostics_dir" ]]; then
  for artifact_class in traces evals exports; do
    artifact_dir="$diagnostics_dir/$artifact_class"
    artifact_dump="$work_dir/diagnostics-$artifact_class.dump"
    if [[ -d "$artifact_dir" ]]; then
      find "$artifact_dir" -type f -exec cat {} + > "$artifact_dump" 2>/dev/null || true
    else
      : > "$artifact_dump"
    fi
  done
fi

if grep -Eiq 'full_message|(^|[^a-z])(body|prompt|response)([^a-z]|$)' "$work_dir/privacy-columns.txt"; then
  echo "privacy inspection failed: forbidden storage column present" >&2
  exit 2
fi

report_forbidden_token() {
  local artifact_class="$1"
  local token_index="$2"
  local token="$3"
  local token_digest
  token_digest="$(printf '%s' "$token" | shasum -a 256 | awk '{print $1}')"
  echo "privacy inspection failed: forbidden token found in ${artifact_class}: index=${token_index} length=${#token} sha256=${token_digest}" >&2
}

token_index=0
while IFS= read -r token; do
  token_index=$((token_index + 1))
  [[ -z "$token" ]] && continue
  for artifact_class in db logs; do
    if grep -Fq "$token" "$work_dir/$artifact_class.dump"; then
      report_forbidden_token "$artifact_class" "$token_index" "$token"
      exit 3
    fi
  done
  if [[ -n "$diagnostics_dir" ]]; then
    for artifact_class in traces evals exports; do
      if grep -Fq "$token" "$work_dir/diagnostics-$artifact_class.dump"; then
        report_forbidden_token "diagnostics/$artifact_class" "$token_index" "$token"
        exit 3
      fi
    done
  fi
done < "$forbidden_tokens"

echo "privacy inspection passed: no full messages, prompts, responses, or forbidden non-whitelisted content found"
