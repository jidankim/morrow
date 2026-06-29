use std::{
    path::{Path, PathBuf},
    process::Command,
};

use morrow_lib::native_bridge::{
    FakeNativeBridge, NativeBridgeState, ScanSelectedChatsRequest, ScanSelectedChatsResult,
};
use morrow_messages::{
    ChatGuid, MessageGuid, MessageTimestamp, NativeBatch, ParticipantId, RawChat, RawMessage,
    TapbackKind,
};
use morrow_storage::{CandidateId, CandidateState, Store};
use serde_json::{json, Value};

pub(super) fn temp_db(name: &str) -> Result<(tempfile::TempDir, PathBuf), String> {
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join(name);
    Ok((dir, db_path))
}

pub(super) fn fake_state(db_path: &Path, messages: NativeBatch) -> NativeBridgeState {
    NativeBridgeState::with_bridge(
        FakeNativeBridge::with_morrow_store_path(db_path.to_path_buf()).with_messages(messages),
    )
}

pub(super) fn assert_counts(
    result: &ScanSelectedChatsResult,
    expected: (usize, usize, usize, usize, usize),
) {
    assert_eq!(
        (
            result.pending_proposal_count,
            result.created_candidate_count,
            result.quiet_log_count,
            result.cap_visible_count,
            result.cap_deferred_count
        ),
        expected
    );
}

pub(super) fn candidate_state(
    store: &Store,
    result: &ScanSelectedChatsResult,
) -> Result<CandidateState, String> {
    let stored = result
        .created_candidate_ids
        .first()
        .ok_or_else(|| "missing created candidate id".to_owned())?;
    let candidate_id = CandidateId::from_storage(stored).map_err(|error| error.to_string())?;
    store
        .candidate_state(&candidate_id)
        .map_err(|error| error.to_string())
}

pub(super) fn scan_storage_dump(db_path: &Path) -> Result<String, String> {
    query_sqlite(
        db_path,
        "\
        SELECT chat_guid || '|' || anchor_message_guid || '|' || title FROM candidates \
        UNION ALL SELECT message_guid || '|' || excerpt FROM evidence \
        UNION ALL SELECT chat_guid || '|' || anchor_message_guid || '|' || excerpt FROM quiet_logs \
        ORDER BY 1;",
    )
}

pub(super) fn native_batch() -> Result<NativeBatch, String> {
    Ok(batch(vec![
        raw_chat(
            "design-partners",
            "msg-design",
            "Let's meet 2026-07-15 14:00 at the private clinic.",
            Some(TapbackKind::Like),
        )?,
        raw_chat(
            "ops-triage",
            "msg-ops",
            "The private clinic status update was funny.",
            None,
        )?,
    ]))
}

pub(super) fn batch(chats: Vec<RawChat>) -> NativeBatch {
    NativeBatch { chats }
}

pub(super) fn raw_chat(
    chat_guid: &str,
    message_guid: &str,
    text: &str,
    tapback: Option<TapbackKind>,
) -> Result<RawChat, String> {
    raw_chat_with_participants(
        chat_guid,
        message_guid,
        3,
        &["p1", "p2", "p3"],
        text,
        tapback,
    )
}

pub(super) fn raw_chat_with_participants(
    chat_guid: &str,
    message_guid: &str,
    participant_count: u16,
    participant_ids: &[&str],
    text: &str,
    tapback: Option<TapbackKind>,
) -> Result<RawChat, String> {
    let chat = ChatGuid::parse(chat_guid).map_err(|error| error.to_string())?;
    Ok(RawChat {
        guid: chat.clone(),
        participant_count,
        participant_ids: participant_ids
            .iter()
            .map(|id| ParticipantId::parse(id).map_err(|error| error.to_string()))
            .collect::<Result<Vec<_>, _>>()?,
        messages: vec![RawMessage {
            chat_guid: chat,
            message_guid: MessageGuid::parse(message_guid).map_err(|error| error.to_string())?,
            timestamp: MessageTimestamp::new(1_782_352_400).map_err(|error| error.to_string())?,
            text: text.to_owned(),
            tapback,
        }],
    })
}

pub(super) fn scan_request(
    selected_chats: &[ChatFixture<'_>],
    backfill_prompt_chat_ids: &[&str],
    source_excerpts_enabled: bool,
    max_visible: usize,
    pending_count: usize,
) -> Result<ScanSelectedChatsRequest, String> {
    request_value(
        json!(selected_chats
            .iter()
            .map(|chat| chat.id)
            .collect::<Vec<_>>()),
        json!(selected_chats
            .iter()
            .map(|chat| selected_chat_json(chat.id, chat.participant_count, chat.participant_ids))
            .collect::<Vec<_>>()),
        backfill_prompt_chat_ids
            .iter()
            .map(|chat_id| (*chat_id).to_owned())
            .collect(),
        source_excerpts_enabled,
        max_visible,
        pending_count,
    )
}

pub(super) fn malformed_request(
    selected_chat_ids: Value,
    selected_chats: Value,
) -> Result<ScanSelectedChatsRequest, String> {
    request_value(selected_chat_ids, selected_chats, Vec::new(), true, 1, 0)
}

fn request_value(
    selected_chat_ids: Value,
    selected_chats: Value,
    backfill_prompt_chat_ids: Vec<String>,
    source_excerpts_enabled: bool,
    max_visible: usize,
    pending_count: usize,
) -> Result<ScanSelectedChatsRequest, String> {
    serde_json::from_value(json!({
        "selectedChatIds": selected_chat_ids,
        "selectedChats": selected_chats,
        "referenceTimezone": "Asia/Seoul",
        "referenceUnixSeconds": 1_782_352_400,
        "backfillPromptChatIds": backfill_prompt_chat_ids,
        "sourceExcerptsEnabled": source_excerpts_enabled,
        "capPolicy": {
            "mode": "refillForPending",
            "maxVisible": max_visible,
            "pendingCount": pending_count,
        },
    }))
    .map_err(|error| error.to_string())
}

pub(super) fn selected_chat_json(
    id: &str,
    participant_count: u16,
    participant_ids: &[&str],
) -> Value {
    json!({
        "id": id,
        "label": "Display label ignored by native scan",
        "participantCount": participant_count,
        "participantIds": participant_ids,
        "latestActivityTimestamp": 1_782_352_400,
    })
}

pub(super) const fn chat<'a>(
    id: &'a str,
    participant_count: u16,
    participant_ids: &'a [&'a str],
) -> ChatFixture<'a> {
    ChatFixture {
        id,
        participant_count,
        participant_ids,
    }
}

#[derive(Clone, Copy)]
pub(super) struct ChatFixture<'a> {
    id: &'a str,
    participant_count: u16,
    participant_ids: &'a [&'a str],
}

fn query_sqlite(db_path: &Path, sql: &str) -> Result<String, String> {
    let output = Command::new("sqlite3")
        .arg("-batch")
        .arg("-noheader")
        .arg(db_path)
        .arg(sql)
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}
