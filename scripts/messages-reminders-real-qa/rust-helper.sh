write_rust_helper() {
  cat > "$example_path" <<'RS'
use std::{path::PathBuf, process::Command};

use morrow_lib::native_bridge::{
    probe_codex_provider_auth, CodexAuthStatus, MessagesDiscoveryCommandReport, NativeBridgeState,
    ScanSelectedChatsRequest,
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
            println!("{kind}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<Vec<String>, &'static str> {
    let chat_id = env_required("MORROW_REAL_QA_CHAT_PUBLIC_ID")?;
    let store_path = PathBuf::from(env_required("MORROW_REAL_QA_STORE_PATH")?);
    let messages_path = std::env::var_os("MORROW_MESSAGES_DB")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Messages/chat.db"))
        })
        .ok_or("STATUS=BLOCKED_FULL_DISK_ACCESS")?;

    let state = NativeBridgeState::default();
    let readiness = probe_codex_provider_auth();
    if !readiness.ready {
        return Ok(vec![match readiness.status {
            CodexAuthStatus::MissingCli => "STATUS=BLOCKED_CODEX_CLI_MISSING".to_owned(),
            CodexAuthStatus::NotLoggedIn => "STATUS=BLOCKED_CODEX_LOGIN_REQUIRED".to_owned(),
            CodexAuthStatus::Timeout => "STATUS=BLOCKED_CODEX_AUTH_TIMEOUT".to_owned(),
            CodexAuthStatus::UnknownFailure | CodexAuthStatus::LoggedInUsingChatGpt => {
                "STATUS=BLOCKED_CODEX_AUTH_UNAVAILABLE".to_owned()
            }
        }]);
    }

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
        "feedbackTextSnapshotsEnabled": false,
        "localDiagnosticsEnabled": false,
        "localDiagnosticsRetentionDays": 30,
        "capPolicy": { "mode": "refillForPending", "maxVisible": 1, "pendingCount": 0 }
    });
    let request: ScanSelectedChatsRequest =
        serde_json::from_value(request_value).map_err(|_| "STATUS=REQUEST_UNAVAILABLE")?;
    let result = match state.scan_selected_chats_at(request, &store_path, &messages_path) {
        Ok(result) => result,
        Err(error) => {
            let message = error.to_string().to_ascii_lowercase();
            if message.contains("permission")
                || message.contains("not authorized")
                || message.contains("unable to open database")
            {
                return Ok(vec!["STATUS=BLOCKED_FULL_DISK_ACCESS".to_owned()]);
            }
            if message.contains("reminders") && message.contains("access") {
                return Ok(vec!["STATUS=BLOCKED_REMINDERS_ACCESS".to_owned()]);
            }
            return Ok(vec![
                "STATUS=SCAN_FAILED".to_owned(),
                format!("SCAN_ERROR={}", sanitized_scan_error(&message)),
            ]);
        }
    };

    let visible_task_reminders = store_count(&store_path, "SELECT COUNT(*) FROM candidates WHERE kind = 'task_reminder' AND state = 'visible';")?;
    let reminder_receipts = store_count(&store_path, "SELECT COUNT(*) FROM candidates WHERE kind = 'task_reminder' AND state = 'visible' AND external_object_id IS NOT NULL AND external_source_id IS NOT NULL;")?;
    let reminder_mappings = store_count(&store_path, "SELECT COUNT(*) FROM candidates c JOIN external_object_mappings m ON m.candidate_id = c.id AND m.source = 'reminders' WHERE c.kind = 'task_reminder' AND c.state = 'visible' AND c.external_object_id IS NOT NULL AND c.external_source_id IS NOT NULL;")?;
    let failed_task_reminders = store_count(&store_path, "SELECT COUNT(*) FROM candidates WHERE kind = 'task_reminder' AND state = 'failed' AND current_reason LIKE 'external_proposal_creation_failed%';")?;
    if reminder_mappings == 0 && failed_task_reminders > 0 {
        return Ok(vec!["STATUS=BLOCKED_REMINDERS_ACCESS".to_owned()]);
    }
    if reminder_mappings == 0 {
        return Ok(vec![
            "STATUS=NO_REMINDER_CREATED".to_owned(),
            format!("CREATED_CANDIDATES={}", result.created_candidate_count),
            format!("QUIET_LOGS={}", result.quiet_log_count),
            format!("VISIBLE_TASK_REMINDERS={visible_task_reminders}"),
            format!("REMINDER_RECEIPTS={reminder_receipts}"),
            "REMINDER_MAPPINGS=0".to_owned(),
        ]);
    }
    Ok(vec![
        "STATUS=SCAN_OK".to_owned(),
        format!("CREATED_EXTERNAL={}", result.created_external_proposal_count),
        format!("FAILED_EXTERNAL={}", result.failed_external_proposal_count),
        format!("CREATED_CANDIDATES={}", result.created_candidate_count),
        format!("QUIET_LOGS={}", result.quiet_log_count),
        format!("VISIBLE_TASK_REMINDERS={visible_task_reminders}"),
        format!("REMINDER_RECEIPTS={reminder_receipts}"),
        format!("REMINDER_MAPPINGS={reminder_mappings}"),
    ])
}

fn env_required(name: &'static str) -> Result<String, &'static str> {
    std::env::var(name).map_err(|_| "STATUS=MISSING_ENV")
}

fn store_count(path: &PathBuf, sql: &str) -> Result<u64, &'static str> {
    let output = Command::new("sqlite3")
        .arg("-batch")
        .arg("-noheader")
        .arg(path)
        .arg(sql)
        .output()
        .map_err(|_| "STATUS=STORE_READBACK_UNAVAILABLE")?;
    if !output.status.success() {
        return Err("STATUS=STORE_READBACK_UNAVAILABLE");
    }
    let raw = String::from_utf8(output.stdout).map_err(|_| "STATUS=STORE_READBACK_UNAVAILABLE")?;
    raw.trim()
        .parse::<u64>()
        .map_err(|_| "STATUS=STORE_READBACK_UNAVAILABLE")
}

fn sanitized_scan_error(message: &str) -> String {
    message
        .chars()
        .filter(|ch| !ch.is_control())
        .take(240)
        .collect()
}
RS
}
