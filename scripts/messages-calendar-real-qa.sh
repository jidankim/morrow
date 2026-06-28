#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
evidence_dir="${MORROW_REAL_QA_EVIDENCE_DIR:-.omo/evidence/messages-calendar-production-wiring/task-7}"
evidence_dir="$repo_root/$evidence_dir"
evidence_root="$repo_root/.omo/evidence/messages-calendar-production-wiring"
storage_db="$evidence_dir/morrow-real-qa.sqlite"
run_log="$evidence_dir/real-qa-run.log"
readback_json="$evidence_dir/real-qa-readback.json"
cleanup_log="$evidence_dir/real-qa-cleanup.log"
privacy_log="$evidence_dir/privacy-scan.log"
blocked_cases_log="$evidence_root/task-7-real-qa-blocked-cases.log"
forbidden_tokens="$evidence_root/forbidden-tokens.txt"
work_dir="$(mktemp -d)"
example_name="morrow_real_qa_$$"
example_path="$repo_root/src-tauri/examples/${example_name}.rs"
eventkit_binary="$work_dir/morrow-real-qa-eventkit"
event_id=""
cleanup_done="false"

cleanup_temp() {
  if [[ -n "$event_id" && "$cleanup_done" != "true" && -x "$eventkit_binary" ]]; then
    cleanup_event "trap" || true
  fi
  rm -f "$example_path"
  rm -rf "$work_dir"
}
trap cleanup_temp EXIT

sql_literal() {
  local value="$1"
  printf "'%s'" "${value//\'/\'\'}"
}

redacted_id() {
  local value="$1"
  local digest
  digest="$(printf '%s' "$value" | shasum -a 256 | awk '{print substr($1,1,12)}')"
  printf 'redacted-%s' "$digest"
}

write_base_evidence() {
  mkdir -p "$evidence_dir" "$evidence_root"
  : > "$run_log"
  : > "$cleanup_log"
  : > "$readback_json"
  rm -f "$privacy_log"
  if [[ ! -f "$forbidden_tokens" ]]; then
    cat > "$forbidden_tokens" <<'TOKENS'
sk-test-secret-should-not-appear
raw-provider-request-payload
raw-provider-response-body
person@example.com
+15555550123
TOKENS
  fi
}

init_storage_db() {
  rm -f "$storage_db"
  sqlite3 "$storage_db" < "$repo_root/crates/morrow-storage/migrations/0001_init.sql"
}

record_run() {
  local key="$1"
  local value="$2"
  printf '%s=%s\n' "$key" "$value" >> "$run_log"
}

run_privacy_scan() {
  local tmp_log="$work_dir/privacy-scan.log"
  rm -f "$privacy_log"
  scripts/privacy-inspect.sh "$storage_db" "$evidence_dir" "$forbidden_tokens" > "$tmp_log" 2>&1
  mv "$tmp_log" "$privacy_log"
}

emit_blocked() {
  local reason="$1"
  write_base_evidence
  if [[ ! -f "$storage_db" ]]; then
    init_storage_db
  fi
  record_run "scenario" "real_messages_to_calendar_qa"
  record_run "outcome" "blocked"
  record_run "blocked_reason" "$reason"
  record_run "message_body_dumped" "false"
  record_run "raw_handles_dumped" "false"
  record_run "api_key_dumped" "false"
  record_run "provider_payload_dumped" "false"
  if run_privacy_scan; then
    :
  else
    printf 'privacy_scan_status=failed\n' >> "$run_log"
  fi
  printf '%s\n' "BLOCKED: $reason" | tee -a "$blocked_cases_log" >/dev/null
  printf 'BLOCKED: %s\n' "$reason"
  exit 0
}

fail_sanitized() {
  local reason="$1"
  record_run "outcome" "fail"
  record_run "failure_reason" "$reason"
  printf 'FAIL messages_calendar_real_qa: %s\n' "$reason" >&2
  exit 1
}

validate_inputs() {
  if [[ -z "${MORROW_REAL_QA_OPENAI_API_KEY:-}" ]]; then
    emit_blocked "missing provider token"
  fi
  if [[ -z "${MORROW_REAL_QA_CHAT_PUBLIC_ID:-}" ||
        -z "${MORROW_REAL_QA_EXPECTED_TITLE_CONTAINS:-}" ||
        -z "${MORROW_REAL_QA_FUTURE_ISO_LOCAL:-}" ]]; then
    emit_blocked "missing required real QA env"
  fi
  if [[ ! "$MORROW_REAL_QA_FUTURE_ISO_LOCAL" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}$ ]]; then
    write_base_evidence
    init_storage_db
    fail_sanitized "invalid future timestamp"
  fi
}

write_rust_helper() {
  cat > "$example_path" <<'RS'
use std::path::PathBuf;

use morrow_lib::native_bridge::{
    MessagesDiscoveryCommandReport, NativeBridgeState, ScanSelectedChatsRequest,
    TokenLookupRequest, TokenWriteRequest, MORROW_KEYCHAIN_SERVICE, MORROW_PROVIDER_TOKEN_KIND,
};
use serde_json::{json, Value};

fn main() {
    match run() {
        Ok(status) => {
            for line in status {
                println!("{line}");
            }
        }
        Err(kind) => {
            println!("STATUS={kind}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<Vec<String>, &'static str> {
    let chat_id = env_required("MORROW_REAL_QA_CHAT_PUBLIC_ID")?;
    let token = env_required("MORROW_REAL_QA_OPENAI_API_KEY")?;
    let store_path = PathBuf::from(env_required("MORROW_REAL_QA_STORE_PATH")?);
    let messages_path = std::env::var_os("MORROW_MESSAGES_DB")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Messages/chat.db")))
        .ok_or("STATUS=BLOCKED_FULL_DISK_ACCESS")?;

    let state = NativeBridgeState::default();
    let lookup = TokenLookupRequest::new(MORROW_KEYCHAIN_SERVICE, MORROW_PROVIDER_TOKEN_KIND);
    let original = state
        .read_morrow_token(lookup.clone())
        .map_err(|_| "STATUS=KEYCHAIN_UNAVAILABLE")?
        .token;
    let restore = TokenRestore {
        lookup: lookup.clone(),
        original,
        armed: true,
    };
    state
        .create_morrow_token(TokenWriteRequest::new(
            MORROW_KEYCHAIN_SERVICE,
            MORROW_PROVIDER_TOKEN_KIND,
            &token,
        ))
        .map_err(|_| "STATUS=KEYCHAIN_UNAVAILABLE")?;

    let report = state
        .discover_messages_chats_at(&messages_path)
        .map_err(|_| "STATUS=BLOCKED_FULL_DISK_ACCESS")?;
    let command_report = serde_json::to_value(MessagesDiscoveryCommandReport::from_report(&report))
        .map_err(|_| "STATUS=DISCOVERY_UNAVAILABLE")?;
    let status = command_report
        .get("status")
        .and_then(Value::as_str)
        .ok_or("STATUS=DISCOVERY_UNAVAILABLE")?;
    if status == "permissionDenied" || status == "unavailable" {
        return Ok(vec!["STATUS=BLOCKED_FULL_DISK_ACCESS".to_owned()]);
    }
    let chats = command_report
        .get("chats")
        .and_then(Value::as_array)
        .ok_or("STATUS=DISCOVERY_UNAVAILABLE")?;
    let Some(chat) = chats
        .iter()
        .find(|candidate| candidate.get("chatId").and_then(Value::as_str) == Some(chat_id.as_str()))
    else {
        return Ok(vec!["STATUS=BLOCKED_TEST_CHAT_NOT_FOUND".to_owned()]);
    };
    let participant_count = chat
        .get("participantCount")
        .and_then(Value::as_u64)
        .ok_or("STATUS=DISCOVERY_UNAVAILABLE")?;
    let participant_ids = chat
        .get("participantIds")
        .and_then(Value::as_array)
        .cloned()
        .ok_or("STATUS=DISCOVERY_UNAVAILABLE")?;

    let request_value = json!({
        "selectedChatIds": [chat_id],
        "selectedChats": [{
            "id": chat_id,
            "participantCount": participant_count,
            "participantIds": participant_ids
        }],
        "referenceTimezone": "Asia/Seoul",
        "backfillPromptChatIds": [chat_id],
        "sourceExcerptsEnabled": false,
        "capPolicy": { "mode": "refillForPending", "maxVisible": 1, "pendingCount": 0 }
    });
    let request: ScanSelectedChatsRequest =
        serde_json::from_value(request_value).map_err(|_| "STATUS=REQUEST_UNAVAILABLE")?;
    let result = match state.scan_selected_chats_at(request, &store_path, &messages_path) {
        Ok(result) => result,
        Err(error) => {
            let message = error.to_string().to_ascii_lowercase();
            if message.contains("permission") || message.contains("not authorized") || message.contains("unable to open database") {
                return Ok(vec!["STATUS=BLOCKED_FULL_DISK_ACCESS".to_owned()]);
            }
            return Err("STATUS=SCAN_FAILED");
        }
    };
    drop(restore);

    if result.created_external_proposal_count == 0 && result.failed_external_proposal_count > 0 {
        return Ok(vec!["STATUS=BLOCKED_CALENDAR_ACCESS".to_owned()]);
    }
    if result.created_external_proposal_count == 0 {
        return Ok(vec![
            "STATUS=NO_EVENT_CREATED".to_owned(),
            format!("CREATED_CANDIDATES={}", result.created_candidate_count),
            format!("QUIET_LOGS={}", result.quiet_log_count),
        ]);
    }
    Ok(vec![
        "STATUS=SCAN_OK".to_owned(),
        format!("CREATED_EXTERNAL={}", result.created_external_proposal_count),
        format!("FAILED_EXTERNAL={}", result.failed_external_proposal_count),
        format!("CREATED_CANDIDATES={}", result.created_candidate_count),
        format!("QUIET_LOGS={}", result.quiet_log_count),
    ])
}

fn env_required(name: &'static str) -> Result<String, &'static str> {
    std::env::var(name).map_err(|_| "STATUS=MISSING_ENV")
}

struct TokenRestore {
    lookup: TokenLookupRequest,
    original: Option<String>,
    armed: bool,
}

impl Drop for TokenRestore {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let state = NativeBridgeState::default();
        let _ = state.delete_morrow_token(self.lookup.clone());
        if let Some(token) = &self.original {
            let _ = state.create_morrow_token(TokenWriteRequest::new(
                MORROW_KEYCHAIN_SERVICE,
                MORROW_PROVIDER_TOKEN_KIND,
                token,
            ));
        }
    }
}
RS
}

write_eventkit_helper() {
  local objc_source="$work_dir/morrow-real-qa-eventkit.m"
  cat > "$objc_source" <<'OBJC'
#import <EventKit/EventKit.h>
#import <Foundation/Foundation.h>

static BOOL authorize(EKEventStore *store) {
  EKAuthorizationStatus status = [EKEventStore authorizationStatusForEntityType:EKEntityTypeEvent];
  if (status == EKAuthorizationStatusDenied || status == EKAuthorizationStatusRestricted) {
    return NO;
  }
  if (status == EKAuthorizationStatusAuthorized) {
    return YES;
  }
#ifdef EKAuthorizationStatusFullAccess
  if (status == EKAuthorizationStatusFullAccess) {
    return YES;
  }
#endif
  __block BOOL granted = NO;
  dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
  if ([store respondsToSelector:@selector(requestFullAccessToEventsWithCompletion:)]) {
    [store requestFullAccessToEventsWithCompletion:^(BOOL accessGranted, NSError *error) {
      (void)error;
      granted = accessGranted;
      dispatch_semaphore_signal(semaphore);
    }];
  } else {
    [store requestAccessToEntityType:EKEntityTypeEvent completion:^(BOOL accessGranted, NSError *error) {
      (void)error;
      granted = accessGranted;
      dispatch_semaphore_signal(semaphore);
    }];
  }
  dispatch_semaphore_wait(semaphore, dispatch_time(DISPATCH_TIME_NOW, 30 * NSEC_PER_SEC));
  return granted;
}

static NSString *local_iso(NSDate *date) {
  NSDateFormatter *formatter = [[NSDateFormatter alloc] init];
  formatter.locale = [NSLocale localeWithLocaleIdentifier:@"en_US_POSIX"];
  formatter.dateFormat = @"yyyy-MM-dd'T'HH:mm:ss";
  return [formatter stringFromDate:date];
}

static int readback(NSString *eventId, NSString *expectedTitle, NSString *expectedStart) {
  EKEventStore *store = [[EKEventStore alloc] init];
  if (!authorize(store)) {
    printf("BLOCKED_CALENDAR_ACCESS\n");
    return 20;
  }
  EKEvent *event = [store eventWithIdentifier:eventId];
  if (event == nil) {
    printf("{\"event_found\":false}\n");
    return 31;
  }
  BOOL titleOk = [event.title rangeOfString:expectedTitle options:NSCaseInsensitiveSearch].location != NSNotFound;
  BOOL startOk = [local_iso(event.startDate) isEqualToString:expectedStart];
  BOOL calendarOk = [event.calendar.title isEqualToString:@"Morrow Proposed"];
  BOOL availabilityOk = event.availability == EKEventAvailabilityFree;
  NSUInteger alarms = event.alarms == nil ? 0 : event.alarms.count;
  NSUInteger attendees = event.attendees == nil ? 0 : event.attendees.count;
  NSUInteger recurrence = event.recurrenceRules == nil ? 0 : event.recurrenceRules.count;
  printf("{\"event_found\":true,\"title_contains_expected\":%s,\"start_matches_expected\":%s,\"calendar_is_morrow_proposed\":%s,\"availability_free\":%s,\"alarms_count\":%lu,\"attendees_count\":%lu,\"recurrence_rule_count\":%lu}\n",
         titleOk ? "true" : "false",
         startOk ? "true" : "false",
         calendarOk ? "true" : "false",
         availabilityOk ? "true" : "false",
         (unsigned long)alarms,
         (unsigned long)attendees,
         (unsigned long)recurrence);
  return titleOk && startOk && calendarOk && availabilityOk && alarms == 0 && attendees == 0 && recurrence == 0 ? 0 : 32;
}

static int cleanup(NSString *eventId) {
  EKEventStore *store = [[EKEventStore alloc] init];
  if (!authorize(store)) {
    printf("BLOCKED_CALENDAR_ACCESS\n");
    return 20;
  }
  EKEvent *event = [store eventWithIdentifier:eventId];
  if (event == nil) {
    printf("CLEANUP_DELETED=false\n");
    return 0;
  }
  NSError *error = nil;
  BOOL removed = [store removeEvent:event span:EKSpanThisEvent commit:YES error:&error];
  (void)error;
  printf("CLEANUP_DELETED=%s\n", removed ? "true" : "false");
  return removed ? 0 : 40;
}

int main(int argc, const char * argv[]) {
  @autoreleasepool {
    if (argc < 3) {
      fprintf(stderr, "usage: eventkit-helper <readback|cleanup> <event-id> [expected-title] [expected-start]\n");
      return 64;
    }
    NSString *mode = [NSString stringWithUTF8String:argv[1]];
    NSString *eventId = [NSString stringWithUTF8String:argv[2]];
    if ([mode isEqualToString:@"readback"]) {
      if (argc != 5) {
        return 64;
      }
      return readback(eventId, [NSString stringWithUTF8String:argv[3]], [NSString stringWithUTF8String:argv[4]]);
    }
    if ([mode isEqualToString:@"cleanup"]) {
      return cleanup(eventId);
    }
    return 64;
  }
}
OBJC
  /usr/bin/clang -fobjc-arc -Wall -Wextra -framework Foundation -framework EventKit "$objc_source" -o "$eventkit_binary" > "$work_dir/eventkit-build.log" 2>&1
}

run_with_timeout() {
  local seconds="$1"
  local output_file="$2"
  shift 2
  "$@" > "$output_file" 2>&1 &
  local command_pid=$!
  (
    sleep "$seconds"
    kill -TERM "$command_pid" 2>/dev/null || true
    sleep 2
    kill -KILL "$command_pid" 2>/dev/null || true
  ) &
  local watchdog_pid=$!
  local status=0
  wait "$command_pid" || status=$?
  kill "$watchdog_pid" 2>/dev/null || true
  wait "$watchdog_pid" 2>/dev/null || true
  if [[ "$status" -eq 143 || "$status" -eq 137 ]]; then
    return 124
  fi
  return "$status"
}

cleanup_event() {
  local source="$1"
  local raw_cleanup="$work_dir/cleanup.out"
  local redacted
  redacted="$(redacted_id "$event_id")"
  if "$eventkit_binary" cleanup "$event_id" > "$raw_cleanup" 2>&1; then
    if grep -q '^CLEANUP_DELETED=true$' "$raw_cleanup"; then
      printf 'cleanup_source=%s\ncleanup_deleted=true\ncreated_event_id=%s\n' "$source" "$redacted" >> "$cleanup_log"
      cleanup_done="true"
      event_id=""
      return 0
    fi
    printf 'cleanup_source=%s\ncleanup_deleted=false\ncreated_event_id=%s\n' "$source" "$redacted" >> "$cleanup_log"
    cleanup_done="true"
    event_id=""
    return 0
  fi
  if grep -q '^BLOCKED_CALENDAR_ACCESS$' "$raw_cleanup"; then
    printf 'cleanup_source=%s\ncleanup_blocked=missing_calendar_access\ncreated_event_id=%s\n' "$source" "$redacted" >> "$cleanup_log"
    return 20
  fi
  printf 'cleanup_source=%s\ncleanup_failed=true\ncreated_event_id=%s\n' "$source" "$redacted" >> "$cleanup_log"
  return 1
}

main() {
  write_base_evidence
  validate_inputs
  init_storage_db
  record_run "scenario" "real_messages_to_calendar_qa"
  record_run "messages_surface" "native_messages_sqlite_selected_chat"
  record_run "provider_surface" "openai_responses_api"
  record_run "calendar_surface" "eventkit_proposal_bridge"
  record_run "message_body_dumped" "false"
  record_run "raw_handles_dumped" "false"
  record_run "api_key_dumped" "false"
  record_run "provider_payload_dumped" "false"

  write_rust_helper
  if ! write_eventkit_helper; then
    fail_sanitized "eventkit helper compile failed"
  fi

  export MORROW_REAL_QA_STORE_PATH="$storage_db"
  export SDKROOT="${SDKROOT:-/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk}"
  export RUSTFLAGS="${RUSTFLAGS:--C linker=/Library/Developer/CommandLineTools/usr/bin/cc}"
  local helper_output="$work_dir/rust-helper.out"
  local timeout_seconds="${MORROW_REAL_QA_TIMEOUT_SECONDS:-90}"
  if ! (cd "$repo_root/src-tauri" && run_with_timeout "$timeout_seconds" "$helper_output" cargo run --quiet --example "$example_name"); then
    if grep -q '^STATUS=BLOCKED_FULL_DISK_ACCESS$' "$helper_output"; then
      emit_blocked "missing Full Disk Access"
    fi
    fail_sanitized "native scan helper failed"
  fi

  if grep -q '^STATUS=BLOCKED_FULL_DISK_ACCESS$' "$helper_output"; then
    emit_blocked "missing Full Disk Access"
  fi
  if grep -q '^STATUS=BLOCKED_TEST_CHAT_NOT_FOUND$' "$helper_output"; then
    emit_blocked "test chat not found"
  fi
  if grep -q '^STATUS=BLOCKED_CALENDAR_ACCESS$' "$helper_output"; then
    emit_blocked "missing Calendar access"
  fi
  if grep -q '^STATUS=NO_EVENT_CREATED$' "$helper_output"; then
    fail_sanitized "no EventKit proposal was created"
  fi
  if ! grep -q '^STATUS=SCAN_OK$' "$helper_output"; then
    fail_sanitized "native scan did not report success"
  fi

  while IFS= read -r line; do
    case "$line" in
      CREATED_EXTERNAL=*|FAILED_EXTERNAL=*|CREATED_CANDIDATES=*|QUIET_LOGS=*)
        printf '%s\n' "$line" | tr '[:upper:]' '[:lower:]' >> "$run_log"
        ;;
    esac
  done < "$helper_output"

  event_id="$(sqlite3 -batch -noheader "$storage_db" "SELECT external_object_id FROM candidates WHERE external_object_id IS NOT NULL ORDER BY updated_at DESC, id DESC LIMIT 1;" | head -n 1)"
  if [[ -z "$event_id" ]]; then
    fail_sanitized "created EventKit identifier missing from storage"
  fi
  local redacted
  redacted="$(redacted_id "$event_id")"
  record_run "created_event_id" "$redacted"

  local readback_tmp="$work_dir/readback.json"
  if ! "$eventkit_binary" readback "$event_id" "$MORROW_REAL_QA_EXPECTED_TITLE_CONTAINS" "$MORROW_REAL_QA_FUTURE_ISO_LOCAL" > "$readback_tmp" 2>&1; then
    if grep -q '^BLOCKED_CALENDAR_ACCESS$' "$readback_tmp"; then
      cleanup_event "readback_blocked" || true
      emit_blocked "missing Calendar access"
    fi
    cp "$readback_tmp" "$readback_json"
    cleanup_event "readback_failed" || true
    fail_sanitized "EventKit readback validation failed"
  fi
  cp "$readback_tmp" "$readback_json"
  record_run "readback" "ok"

  if ! cleanup_event "pass"; then
    fail_sanitized "EventKit cleanup failed"
  fi
  record_run "cleanup" "ok"

  if ! run_privacy_scan; then
    fail_sanitized "privacy scan failed"
  fi
  record_run "privacy_scan" "ok"
  cp "$run_log" "$evidence_root/task-7-real-qa-pass.log"
  cp "$cleanup_log" "$evidence_root/task-7-real-qa-cleanup.log"
  cp "$privacy_log" "$evidence_root/task-7-privacy-scan.log"
  printf 'PASS created_event_id=%s readback=ok cleanup=ok\n' "$redacted"
}

main "$@"
