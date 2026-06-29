#[path = "native_scan/dependencies.rs"]
mod dependencies;

use dependencies::{CandidateProvider, RecordingProposalAdapter, UnavailableTestProvider};
use morrow_lib::native_bridge::{
    scan_selected_chats_with_dependencies, FakeNativeBridge, NativeBridgeState,
    ScanSelectedChatsDependencies, ScanSelectedChatsRequest, ScanSelectedChatsResult,
};
use morrow_messages::{
    ChatGuid, MessageGuid, MessageTimestamp, NativeBatch, ParticipantId, RawChat, RawMessage,
    TapbackKind,
};
use morrow_storage::{CandidateId, CandidateState, ReplayStream, Store};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[test]
#[rustfmt::skip]
fn scan_selected_chats_hides_source_excerpts_and_applies_caps() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan.sqlite")?;
    let request = scan_request(&[chat("design-partners", 3, &["p1", "p2", "p3"]), chat("ops-triage", 3, &["p1", "p2", "p3"])], &["design-partners"], false, 1, 0)?;
    // When
    let result = fake_state(&db_path, native_batch()?).scan_selected_chats_at(request, &db_path, &db_path).map_err(|error| error.to_string())?;
    // Then
    assert_counts(&result, (1, 1, 1, 1, 0));
    let store = Store::open(&db_path).map_err(|error| error.to_string())?;
    assert_eq!(candidate_state(&store, &result)?, CandidateState::Visible);
    assert!(store.next_replay_candidates(ReplayStream::CalendarProposals, 10).map_err(|error| error.to_string())?.is_empty());
    assert_eq!(store.privacy_summary().map_err(|error| error.to_string())?.max_excerpt_len, "Source excerpt hidden by settings.".len());
    Ok(())
}

#[test]
#[rustfmt::skip]
fn scan_selected_chats_redacts_native_anchors_and_excerpts_in_storage() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-redaction.sqlite")?;
    let request = scan_request(&[chat("design-partners", 3, &["p1", "p2", "p3"]), chat("ops-triage", 3, &["p1", "p2", "p3"])], &[ "design-partners" ], true, 1, 0)?;
    // When
    let result = fake_state(&db_path, native_batch()?).scan_selected_chats_at(request, &db_path, &db_path).map_err(|error| error.to_string())?;
    // Then
    assert_counts(&result, (1, 1, 1, 1, 0));
    let stored = scan_storage_dump(&db_path)?;
    assert!(stored.contains("messages-chat-"), "{stored}");
    assert!(stored.contains("messages-message-"), "{stored}");
    assert!(stored.contains("Source excerpt hidden by settings."), "{stored}");
    for forbidden in ["design-partners", "ops-triage", "msg-design", "msg-ops", "Let's meet 2026-07-15 14:00 at the private clinic.", "The private clinic status update was funny."] {
        assert!(!stored.contains(forbidden), "stored native scan data leaked {forbidden}: {stored}");
    }
    Ok(())
}

#[test]
#[rustfmt::skip]
fn scan_selected_chats_refill_policy_defers_when_pending_fills_cap() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-refill.sqlite")?;
    let request = scan_request(&[chat("design-partners", 3, &["p1", "p2", "p3"])], &[], true, 1, 1)?;
    // When
    let result = fake_state(&db_path, native_batch()?).scan_selected_chats_at(request, &db_path, &db_path).map_err(|error| error.to_string())?;
    // Then
    assert_counts(&result, (1, 1, 0, 0, 1));
    let store = Store::open(&db_path).map_err(|error| error.to_string())?;
    assert_eq!(candidate_state(&store, &result)?, CandidateState::Queued);
    Ok(())
}

#[test]
#[rustfmt::skip]
fn scan_selected_chats_records_degraded_message_boundary_without_candidates() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-unavailable.sqlite")?;
    let state = NativeBridgeState::with_bridge(FakeNativeBridge::with_morrow_store_path(db_path.clone()));
    let request = scan_request(&[chat("design-partners", 3, &["p1", "p2", "p3"])], &[], true, 1, 0)?;
    // When
    let result = state.scan_selected_chats_at(request, &db_path, &db_path).map_err(|error| error.to_string())?;
    // Then
    assert_counts(&result, (0, 0, 0, 0, 0));
    Ok(())
}

#[test]
#[rustfmt::skip]
fn scan_selected_chats_honors_selected_participant_metadata() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-participants.sqlite")?;
    let messages = batch(vec![raw_chat_with_participants("two-person-design", "msg-two-person", 2, &["local-a", "local-b"], "Let's meet 2026-07-15 14:00 at the private clinic.", Some(TapbackKind::Like))?]);
    let request = scan_request(&[chat("two-person-design", 2, &["local-a", "local-b"])], &[], true, 1, 0)?;
    // When
    let result = fake_state(&db_path, messages).scan_selected_chats_at(request, &db_path, &db_path).map_err(|error| error.to_string())?;
    // Then
    assert_counts(&result, (1, 1, 0, 1, 0));
    Ok(())
}

#[test]
#[rustfmt::skip]
fn scan_selected_chats_pauses_when_same_count_participant_ids_change() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-participant-swap.sqlite")?;
    let messages = batch(vec![raw_chat_with_participants("design-partners", "msg-design", 3, &["p1", "p2", "p4"], "Let's meet 2026-07-15 14:00 at the private clinic.", Some(TapbackKind::Like))?]);
    let request = scan_request(&[chat("design-partners", 3, &["p1", "p2", "p3"])], &[], true, 1, 0)?;
    // When
    let result = fake_state(&db_path, messages).scan_selected_chats_at(request, &db_path, &db_path).map_err(|error| error.to_string())?;
    // Then
    assert_counts(&result, (0, 0, 0, 0, 0));
    Ok(())
}

#[test]
#[rustfmt::skip]
fn production_scan_reads_messages_before_provider_unavailable_boundary() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let store_path = dir.path().join("morrow.sqlite");
    let messages_db_path = dir.path().join("chat.db");
    create_messages_fixture(&messages_db_path)?;
    let request = scan_request(&[chat("messages-chat-8b96e568c027a42d3b5c9e6e7710201f", 1, &["messages-participant-6044b729eea9fa126e78d421e4a41ac8"])], &[], true, 1, 0)?;
    // When
    let result = NativeBridgeState::default().scan_selected_chats_at(request, &store_path, &messages_db_path).map_err(|error| error.to_string())?;
    // Then
    assert_counts(&result, (0, 0, 1, 0, 0));
    Ok(())
}

#[test]
#[rustfmt::skip]
fn production_scan_uses_provider_and_proposal_adapter_dependencies() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let store_path = dir.path().join("morrow.sqlite");
    let messages_db_path = dir.path().join("chat.db");
    create_messages_fixture(&messages_db_path)?;
    let source = morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter::new(messages_db_path);
    let request = scan_request(&[chat("iMessage;-;+15555550103", 1, &["messages-participant-6044b729eea9fa126e78d421e4a41ac8"])], &[], true, 1, 0)?;
    let provider = CandidateProvider;
    let adapter = RecordingProposalAdapter::default();

    // When
    let result = scan_selected_chats_with_dependencies(
        request,
        &store_path,
        ScanSelectedChatsDependencies {
            source: &source,
            provider: &provider,
            proposal_adapter: &adapter,
        },
    ).map_err(|error| error.to_string())?;

    // Then
    assert_counts(&result, (1, 1, 0, 1, 0));
    assert_eq!(result.created_external_proposal_count, 1);
    assert_eq!(adapter.created_titles(), ["Provider supplied title"]);
    let store = Store::open(&store_path).map_err(|error| error.to_string())?;
    assert_eq!(candidate_state(&store, &result)?, CandidateState::Visible);
    Ok(())
}

#[test]
#[rustfmt::skip]
fn production_scan_uses_eventkit_bridge_for_visible_calendar_candidates() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let store_path = dir.path().join("morrow.sqlite");
    let messages_db_path = dir.path().join("chat.db");
    create_messages_fixture(&messages_db_path)?;
    let source = morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter::new(messages_db_path);
    let request = scan_request(&[chat("iMessage;-;+15555550103", 1, &["messages-participant-6044b729eea9fa126e78d421e4a41ac8"])], &[], true, 1, 0)?;
    let provider = CandidateProvider;
    let adapter = RecordingProposalAdapter::default();

    // When
    let result = scan_selected_chats_with_dependencies(
        request,
        &store_path,
        ScanSelectedChatsDependencies {
            source: &source,
            provider: &provider,
            proposal_adapter: &adapter,
        },
    ).map_err(|error| error.to_string())?;

    // Then
    assert_counts(&result, (1, 1, 0, 1, 0));
    assert_eq!(result.created_external_proposal_count, 1);
    assert_eq!(result.failed_external_proposal_count, 0);
    assert_eq!(adapter.created_titles(), ["Provider supplied title"]);
    let store = Store::open(&store_path).map_err(|error| error.to_string())?;
    assert_eq!(candidate_state(&store, &result)?, CandidateState::Visible);
    assert_eq!(external_mapping_count(&store_path)?, 1);
    assert_eq!(candidate_external_receipt_count(&store_path)?, 1);
    Ok(())
}

#[test]
#[rustfmt::skip]
fn eventkit_replay_failure_records_privacy_safe_failure_state() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let store_path = dir.path().join("morrow.sqlite");
    let messages_db_path = dir.path().join("chat.db");
    create_messages_fixture(&messages_db_path)?;
    let source = morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter::new(messages_db_path);
    let request = scan_request(&[chat("iMessage;-;+15555550103", 1, &["messages-participant-6044b729eea9fa126e78d421e4a41ac8"])], &[], true, 1, 0)?;
    let provider = CandidateProvider;
    let adapter = RecordingProposalAdapter::failing_calendar();

    // When
    let result = scan_selected_chats_with_dependencies(
        request,
        &store_path,
        ScanSelectedChatsDependencies {
            source: &source,
            provider: &provider,
            proposal_adapter: &adapter,
        },
    ).map_err(|error| error.to_string())?;

    // Then
    assert_counts(&result, (1, 1, 0, 1, 0));
    assert_eq!(result.created_external_proposal_count, 0);
    assert_eq!(result.failed_external_proposal_count, 1);
    let store = Store::open(&store_path).map_err(|error| error.to_string())?;
    assert_eq!(candidate_state(&store, &result)?, CandidateState::Failed);
    let reasons = candidate_reasons(&store_path)?;
    assert!(reasons.contains("external_proposal_creation_failed"), "{reasons}");
    for forbidden in ["+15555550103", "Maybe meet tomorrow?", "beta-provider-route"] {
        assert!(!reasons.contains(forbidden), "failure state leaked {forbidden}: {reasons}");
    }
    Ok(())
}

#[test]
#[rustfmt::skip]
fn eventkit_replay_retry_does_not_duplicate_after_storage_failure() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let store_path = dir.path().join("morrow.sqlite");
    let messages_db_path = dir.path().join("chat.db");
    create_messages_fixture(&messages_db_path)?;
    let _store = Store::open(&store_path).map_err(|error| error.to_string())?;
    install_external_mapping_failure_trigger(&store_path)?;
    let source = morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter::new(messages_db_path);
    let request = scan_request(&[chat("iMessage;-;+15555550103", 1, &["messages-participant-6044b729eea9fa126e78d421e4a41ac8"])], &[], true, 1, 0)?;
    let provider = CandidateProvider;
    let adapter = RecordingProposalAdapter::default();

    // When
    let first = scan_selected_chats_with_dependencies(
        request.clone(),
        &store_path,
        ScanSelectedChatsDependencies {
            source: &source,
            provider: &provider,
            proposal_adapter: &adapter,
        },
    ).map_err(|error| error.to_string())?;
    drop_external_mapping_failure_trigger(&store_path)?;
    let second = scan_selected_chats_with_dependencies(
        request,
        &store_path,
        ScanSelectedChatsDependencies {
            source: &source,
            provider: &provider,
            proposal_adapter: &adapter,
        },
    ).map_err(|error| error.to_string())?;

    // Then
    assert_eq!(first.created_external_proposal_count, 0);
    assert_eq!(first.failed_external_proposal_count, 1);
    assert_eq!(second.created_external_proposal_count, 0);
    assert_eq!(second.failed_external_proposal_count, 0);
    assert_eq!(adapter.created_count(), 1);
    assert_eq!(external_mapping_count(&store_path)?, 1);
    assert_eq!(candidate_external_receipt_count(&store_path)?, 1);
    let store = Store::open(&store_path).map_err(|error| error.to_string())?;
    assert_eq!(candidate_state(&store, &first)?, CandidateState::Visible);
    Ok(())
}

#[test]
#[rustfmt::skip]
fn injected_unavailable_provider_records_quiet_failure_without_candidates() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-injected-unavailable.sqlite")?;
    let source = FakeNativeBridge::with_morrow_store_path(db_path.clone()).with_messages(batch(vec![raw_chat("design-partners", "msg-provider-route", "Maybe meet tomorrow?", Some(TapbackKind::Like))?]));
    let request = scan_request(&[chat("design-partners", 3, &["p1", "p2", "p3"])], &[], true, 1, 0)?;
    let provider = UnavailableTestProvider;
    let adapter = RecordingProposalAdapter::default();

    // When
    let result = scan_selected_chats_with_dependencies(
        request,
        &db_path,
        ScanSelectedChatsDependencies {
            source: &source,
            provider: &provider,
            proposal_adapter: &adapter,
        },
    ).map_err(|error| error.to_string())?;

    // Then
    assert_counts(&result, (0, 0, 1, 0, 0));
    assert_eq!(result.created_external_proposal_count, 0);
    assert_eq!(adapter.created_titles().len(), 0);
    Ok(())
}

#[test]
#[rustfmt::skip]
fn scan_selected_chats_rejects_malformed_selected_metadata() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-malformed.sqlite")?;
    let state = fake_state(&db_path, NativeBatch::default());
    let cases = [
        ("empty selected list", json!([]), json!([]), "selected_chats"),
        ("mismatched selected ids", json!(["chat-a"]), json!([selected_chat_json("chat-b", 1, &["p1"])]), "selected_chats"),
        ("duplicate participant ids", json!(["chat-a"]), json!([selected_chat_json("chat-a", 2, &["p1", "p1"])]), "participant_ids"),
        ("invalid guid", json!([""]), json!([selected_chat_json("", 1, &["p1"])]), "chat_guid"),
    ];
    for (name, selected_chat_ids, selected_chats, expected) in cases {
        // When
        let error = state.scan_selected_chats_at(malformed_request(selected_chat_ids, selected_chats)?, &db_path, &db_path).err().ok_or_else(|| format!("{name}: scan unexpectedly succeeded"))?.to_string();
        // Then
        assert!(error.contains(expected), "{name}: {error}");
    }
    Ok(())
}

#[test]
#[rustfmt::skip]
fn scan_selected_chats_rejects_unsupported_reference_timezone() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-timezone.sqlite")?;
    let state = fake_state(&db_path, native_batch()?);
    let mut request = scan_request(&[chat("design-partners", 3, &["p1", "p2", "p3"])], &[], true, 1, 0)?;
    request.reference_timezone = "America/Los_Angeles".to_owned();

    // When
    let error = state.scan_selected_chats_at(request, &db_path, &db_path).err().ok_or_else(|| "scan unexpectedly succeeded".to_owned())?.to_string();

    // Then
    assert!(error.contains("unsupported reference timezone"), "{error}");
    assert!(!error.contains("America/Los_Angeles"), "{error}");
    Ok(())
}

#[rustfmt::skip]
fn temp_db(name: &str) -> Result<(tempfile::TempDir, PathBuf), String> {
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join(name);
    Ok((dir, db_path))
}

#[rustfmt::skip]
fn fake_state(db_path: &Path, messages: NativeBatch) -> NativeBridgeState {
    NativeBridgeState::with_bridge(FakeNativeBridge::with_morrow_store_path(db_path.to_path_buf()).with_messages(messages))
}

#[rustfmt::skip]
fn assert_counts(result: &ScanSelectedChatsResult, expected: (usize, usize, usize, usize, usize)) {
    assert_eq!((result.pending_proposal_count, result.created_candidate_count, result.quiet_log_count, result.cap_visible_count, result.cap_deferred_count), expected);
}

#[rustfmt::skip]
fn candidate_state(store: &Store, result: &ScanSelectedChatsResult) -> Result<CandidateState, String> {
    let stored = result.created_candidate_ids.first().ok_or_else(|| "missing created candidate id".to_owned())?;
    let candidate_id = CandidateId::from_storage(stored).map_err(|error| error.to_string())?;
    store.candidate_state(&candidate_id).map_err(|error| error.to_string())
}

#[rustfmt::skip]
fn scan_storage_dump(db_path: &Path) -> Result<String, String> {
    query_sqlite(db_path, "SELECT chat_guid || '|' || anchor_message_guid || '|' || title FROM candidates UNION ALL SELECT message_guid || '|' || excerpt FROM evidence UNION ALL SELECT chat_guid || '|' || anchor_message_guid || '|' || excerpt FROM quiet_logs ORDER BY 1;")
}

#[rustfmt::skip]
fn native_batch() -> Result<NativeBatch, String> {
    Ok(batch(vec![
        raw_chat("design-partners", "msg-design", "Let's meet 2026-07-15 14:00 at the private clinic.", Some(TapbackKind::Like))?,
        raw_chat("ops-triage", "msg-ops", "The private clinic status update was funny.", None)?,
    ]))
}

#[rustfmt::skip]
fn batch(chats: Vec<RawChat>) -> NativeBatch {
    NativeBatch { chats }
}

#[rustfmt::skip]
fn raw_chat(chat_guid: &str, message_guid: &str, text: &str, tapback: Option<TapbackKind>) -> Result<RawChat, String> {
    raw_chat_with_participants(chat_guid, message_guid, 3, &["p1", "p2", "p3"], text, tapback)
}

#[rustfmt::skip]
fn raw_chat_with_participants(chat_guid: &str, message_guid: &str, participant_count: u16, participant_ids: &[&str], text: &str, tapback: Option<TapbackKind>) -> Result<RawChat, String> {
    let chat = ChatGuid::parse(chat_guid).map_err(|error| error.to_string())?;
    Ok(RawChat {
        guid: chat.clone(),
        participant_count,
        participant_ids: participant_ids.iter().map(|id| ParticipantId::parse(id).map_err(|error| error.to_string())).collect::<Result<Vec<_>, _>>()?,
        messages: vec![RawMessage { chat_guid: chat, message_guid: MessageGuid::parse(message_guid).map_err(|error| error.to_string())?, timestamp: MessageTimestamp::new(1_782_352_400).map_err(|error| error.to_string())?, text: text.to_owned(), tapback }],
    })
}

#[rustfmt::skip]
fn scan_request(selected_chats: &[ChatFixture<'_>], backfill_prompt_chat_ids: &[&str], source_excerpts_enabled: bool, max_visible: usize, pending_count: usize) -> Result<ScanSelectedChatsRequest, String> {
    request_value(
        json!(selected_chats.iter().map(|chat| chat.id).collect::<Vec<_>>()),
        json!(selected_chats.iter().map(|chat| selected_chat_json(chat.id, chat.participant_count, chat.participant_ids)).collect::<Vec<_>>()),
        backfill_prompt_chat_ids.iter().map(|chat_id| (*chat_id).to_owned()).collect(),
        source_excerpts_enabled,
        max_visible,
        pending_count,
    )
}

#[rustfmt::skip]
fn malformed_request(selected_chat_ids: Value, selected_chats: Value) -> Result<ScanSelectedChatsRequest, String> {
    request_value(selected_chat_ids, selected_chats, Vec::new(), true, 1, 0)
}

#[rustfmt::skip]
fn request_value(selected_chat_ids: Value, selected_chats: Value, backfill_prompt_chat_ids: Vec<String>, source_excerpts_enabled: bool, max_visible: usize, pending_count: usize) -> Result<ScanSelectedChatsRequest, String> {
    serde_json::from_value(json!({ "selectedChatIds": selected_chat_ids, "selectedChats": selected_chats, "referenceTimezone": "Asia/Seoul", "referenceUnixSeconds": 1_782_352_400, "backfillPromptChatIds": backfill_prompt_chat_ids, "sourceExcerptsEnabled": source_excerpts_enabled, "capPolicy": { "mode": "refillForPending", "maxVisible": max_visible, "pendingCount": pending_count } })).map_err(|error| error.to_string())
}

#[rustfmt::skip]
fn selected_chat_json(id: &str, participant_count: u16, participant_ids: &[&str]) -> Value {
    json!({ "id": id, "label": "Display label ignored by native scan", "participantCount": participant_count, "participantIds": participant_ids, "latestActivityTimestamp": 1_782_352_400 })
}

#[rustfmt::skip]
const fn chat<'a>(id: &'a str, participant_count: u16, participant_ids: &'a [&'a str]) -> ChatFixture<'a> {
    ChatFixture { id, participant_count, participant_ids }
}

#[derive(Clone, Copy)]
struct ChatFixture<'a> {
    id: &'a str,
    participant_count: u16,
    participant_ids: &'a [&'a str],
}

#[rustfmt::skip]
fn create_messages_fixture(db_path: &Path) -> Result<(), String> {
    run_sqlite(db_path, &format!("CREATE TABLE chat (ROWID INTEGER PRIMARY KEY, guid TEXT NOT NULL, display_name TEXT); CREATE TABLE handle (ROWID INTEGER PRIMARY KEY, id TEXT NOT NULL); CREATE TABLE message (ROWID INTEGER PRIMARY KEY, guid TEXT NOT NULL, date INTEGER NOT NULL, text TEXT, attributedBody BLOB, handle_id INTEGER); CREATE TABLE chat_message_join (chat_id INTEGER NOT NULL, message_id INTEGER NOT NULL); CREATE TABLE chat_handle_join (chat_id INTEGER NOT NULL, handle_id INTEGER NOT NULL); INSERT INTO chat (ROWID, guid, display_name) VALUES (1, 'iMessage;-;+15555550103', 'Messages chat'); INSERT INTO handle (ROWID, id) VALUES (3, '+15555550103'); INSERT INTO chat_handle_join (chat_id, handle_id) VALUES (1, 3); INSERT INTO message (ROWID, guid, date, text, attributedBody, handle_id) VALUES (1, 'beta-provider-route', {}, 'Maybe meet tomorrow?', NULL, 3); INSERT INTO chat_message_join (chat_id, message_id) VALUES (1, 1);", apple_nanoseconds(1_782_352_400)))
}

#[rustfmt::skip]
fn run_sqlite(db_path: &Path, sql: &str) -> Result<(), String> {
    let output = Command::new("sqlite3").arg(db_path).arg(sql).output().map_err(|error| error.to_string())?;
    if output.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&output.stderr).trim().to_owned()) }
}

#[rustfmt::skip]
fn query_sqlite(db_path: &Path, sql: &str) -> Result<String, String> {
    let output = Command::new("sqlite3").arg("-batch").arg("-noheader").arg(db_path).arg(sql).output().map_err(|error| error.to_string())?;
    if output.status.success() { Ok(String::from_utf8_lossy(&output.stdout).to_string()) } else { Err(String::from_utf8_lossy(&output.stderr).trim().to_owned()) }
}

#[rustfmt::skip]
fn query_sqlite_i64(db_path: &Path, sql: &str) -> Result<i64, String> {
    query_sqlite(db_path, sql)?.trim().parse::<i64>().map_err(|error| error.to_string())
}

#[rustfmt::skip]
fn external_mapping_count(db_path: &Path) -> Result<i64, String> {
    query_sqlite_i64(db_path, "SELECT COUNT(*) FROM external_object_mappings WHERE source = 'calendar';")
}

#[rustfmt::skip]
fn candidate_external_receipt_count(db_path: &Path) -> Result<i64, String> {
    query_sqlite_i64(db_path, "SELECT COUNT(*) FROM candidates WHERE external_object_id IS NOT NULL AND external_source_id IS NOT NULL;")
}

#[rustfmt::skip]
fn candidate_reasons(db_path: &Path) -> Result<String, String> {
    query_sqlite(db_path, "SELECT current_reason FROM candidates UNION ALL SELECT reason FROM audit_log ORDER BY 1;")
}

#[rustfmt::skip]
fn install_external_mapping_failure_trigger(db_path: &Path) -> Result<(), String> {
    run_sqlite(db_path, "CREATE TRIGGER fail_external_mapping_insert BEFORE INSERT ON external_object_mappings BEGIN SELECT RAISE(FAIL, 'simulated post-create storage failure'); END;")
}

#[rustfmt::skip]
fn drop_external_mapping_failure_trigger(db_path: &Path) -> Result<(), String> {
    run_sqlite(db_path, "DROP TRIGGER fail_external_mapping_insert;")
}

#[rustfmt::skip]
const fn apple_nanoseconds(unix_seconds: i64) -> i64 {
    (unix_seconds - 978_307_200) * 1_000_000_000
}
