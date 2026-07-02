write_rust_helper() {
  cat > "$example_path" <<'RS'
use std::path::PathBuf;

use morrow_lib::native_bridge::{
    MessagesDiscoveryCommandReport, NativeBridgeState, ScanSelectedChatsRequest,
    CodexAuthStatus, probe_codex_provider_auth,
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
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Messages/chat.db")))
        .ok_or("STATUS=BLOCKED_FULL_DISK_ACCESS")?;

    let state = NativeBridgeState::default();
    let readiness = probe_codex_provider_auth();
    if !readiness.ready {
        return Ok(vec![match readiness.status {
            CodexAuthStatus::MissingCli => "STATUS=BLOCKED_CODEX_CLI_MISSING".to_owned(),
            CodexAuthStatus::NotLoggedIn => "STATUS=BLOCKED_CODEX_LOGIN_REQUIRED".to_owned(),
            CodexAuthStatus::Timeout => "STATUS=BLOCKED_CODEX_AUTH_TIMEOUT".to_owned(),
            CodexAuthStatus::UnknownFailure => "STATUS=BLOCKED_CODEX_AUTH_UNAVAILABLE".to_owned(),
            CodexAuthStatus::LoggedInUsingChatGpt => "STATUS=BLOCKED_CODEX_AUTH_UNAVAILABLE".to_owned(),
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
            if message.contains("permission") || message.contains("not authorized") || message.contains("unable to open database") {
                return Ok(vec!["STATUS=BLOCKED_FULL_DISK_ACCESS".to_owned()]);
            }
            return Ok(vec![
                "STATUS=SCAN_FAILED".to_owned(),
                format!("SCAN_ERROR={}", sanitized_scan_error(&message)),
            ]);
        }
    };

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

fn sanitized_scan_error(message: &str) -> String {
    message
        .chars()
        .filter(|ch| !ch.is_control())
        .take(240)
        .collect()
}
RS
}
