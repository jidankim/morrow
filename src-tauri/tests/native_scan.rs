use morrow_lib::native_bridge::{
    CapPolicyRequest, FakeNativeBridge, NativeBridgeState, ScanSelectedChatsRequest,
};
use morrow_messages::{
    ChatGuid, MessageGuid, MessageTimestamp, NativeBatch, RawChat, RawMessage, TapbackKind,
};
use morrow_storage::{CandidateId, CandidateState, ReplayStream, Store};

#[test]
fn scan_selected_chats_hides_source_excerpts_and_applies_caps() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join("native-scan.sqlite");
    let state = NativeBridgeState::with_bridge(
        FakeNativeBridge::with_morrow_store_path(db_path.clone()).with_messages(native_batch()?),
    );
    let request = ScanSelectedChatsRequest {
        selected_chat_ids: vec!["design-partners".to_owned(), "ops-triage".to_owned()],
        reference_timezone: "Asia/Seoul".to_owned(),
        backfill_prompt_chat_ids: vec!["design-partners".to_owned()],
        source_excerpts_enabled: false,
        cap_policy: CapPolicyRequest::RefillForPending {
            max_visible: 1,
            pending_count: 0,
        },
    };

    // When
    let result = state
        .scan_selected_chats_at(request, &db_path)
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(result.pending_proposal_count, 1);
    assert_eq!(result.created_candidate_count, 1);
    assert_eq!(result.quiet_log_count, 1);
    assert_eq!(result.cap_visible_count, 1);
    assert_eq!(result.cap_deferred_count, 0);

    let store = Store::open(&db_path).map_err(|error| error.to_string())?;
    let candidate_id = CandidateId::from_storage(
        result
            .created_candidate_ids
            .first()
            .ok_or_else(|| "missing created candidate id".to_owned())?,
    )
    .map_err(|error| error.to_string())?;
    assert_eq!(
        store
            .candidate_state(&candidate_id)
            .map_err(|error| error.to_string())?,
        CandidateState::Visible
    );
    assert_eq!(
        store
            .next_replay_candidates(ReplayStream::CalendarProposals, 10)
            .map_err(|error| error.to_string())?,
        Vec::<CandidateId>::new()
    );
    let summary = store.privacy_summary().map_err(|error| error.to_string())?;
    assert_eq!(
        summary.max_excerpt_len,
        "Source excerpt hidden by settings.".len()
    );
    Ok(())
}

#[test]
fn scan_selected_chats_refill_policy_defers_when_pending_fills_cap() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join("native-scan-refill.sqlite");
    let state = NativeBridgeState::with_bridge(
        FakeNativeBridge::with_morrow_store_path(db_path.clone()).with_messages(native_batch()?),
    );
    let request = ScanSelectedChatsRequest {
        selected_chat_ids: vec!["design-partners".to_owned()],
        reference_timezone: "Asia/Seoul".to_owned(),
        backfill_prompt_chat_ids: Vec::new(),
        source_excerpts_enabled: true,
        cap_policy: CapPolicyRequest::RefillForPending {
            max_visible: 1,
            pending_count: 1,
        },
    };

    // When
    let result = state
        .scan_selected_chats_at(request, &db_path)
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(result.pending_proposal_count, 1);
    assert_eq!(result.created_candidate_count, 1);
    assert_eq!(result.cap_visible_count, 0);
    assert_eq!(result.cap_deferred_count, 1);

    let store = Store::open(&db_path).map_err(|error| error.to_string())?;
    let candidate_id = CandidateId::from_storage(
        result
            .created_candidate_ids
            .first()
            .ok_or_else(|| "missing deferred candidate id".to_owned())?,
    )
    .map_err(|error| error.to_string())?;
    assert_eq!(
        store
            .candidate_state(&candidate_id)
            .map_err(|error| error.to_string())?,
        CandidateState::Queued
    );
    Ok(())
}

#[test]
fn scan_selected_chats_records_degraded_message_boundary_without_candidates() -> Result<(), String>
{
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join("native-scan-unavailable.sqlite");
    let state =
        NativeBridgeState::with_bridge(FakeNativeBridge::with_morrow_store_path(db_path.clone()));
    let request = ScanSelectedChatsRequest {
        selected_chat_ids: vec!["design-partners".to_owned()],
        reference_timezone: "Asia/Seoul".to_owned(),
        backfill_prompt_chat_ids: Vec::new(),
        source_excerpts_enabled: true,
        cap_policy: CapPolicyRequest::RefillForPending {
            max_visible: 1,
            pending_count: 0,
        },
    };

    // When
    let result = state
        .scan_selected_chats_at(request, &db_path)
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(result.pending_proposal_count, 0);
    assert_eq!(result.created_candidate_count, 0);
    assert_eq!(result.quiet_log_count, 0);
    assert_eq!(result.cap_visible_count, 0);
    assert_eq!(result.cap_deferred_count, 0);
    Ok(())
}

fn native_batch() -> Result<NativeBatch, String> {
    Ok(NativeBatch {
        chats: vec![
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
        ],
    })
}

fn raw_chat(
    chat_guid: &str,
    message_guid: &str,
    text: &str,
    tapback: Option<TapbackKind>,
) -> Result<RawChat, String> {
    let chat = ChatGuid::parse(chat_guid).map_err(|error| error.to_string())?;
    Ok(RawChat {
        guid: chat.clone(),
        participant_count: 3,
        participant_ids: Vec::new(),
        messages: vec![RawMessage {
            chat_guid: chat,
            message_guid: MessageGuid::parse(message_guid).map_err(|error| error.to_string())?,
            timestamp: MessageTimestamp::new(1_782_352_400).map_err(|error| error.to_string())?,
            text: text.to_owned(),
            tapback,
        }],
    })
}
