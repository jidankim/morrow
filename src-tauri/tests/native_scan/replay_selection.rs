use morrow_lib::native_bridge::{
    scan_selected_chats_with_dependencies, ScanSelectedChatsDependencies,
};
use morrow_messages::TapbackKind;
use morrow_storage::{
    CandidateDraft, CandidateKind, CandidateState, ExternalObjectMapping, ExternalSource,
    ReplayStream, Store,
};

use super::dependencies::{RecordingProposalAdapter, UnavailableTestProvider};
use super::support::{assert_counts, batch, chat, raw_chat, scan_request, temp_db};

#[test]
fn replay_selection_omits_recoverable_candidates_already_visible() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-replay-selection.sqlite")?;
    let source =
        morrow_lib::native_bridge::FakeNativeBridge::with_morrow_store_path(db_path.clone())
            .with_messages(batch(vec![raw_chat(
                "design-partners",
                "msg-replay-selection",
                "Let's meet 2026-07-15 14:00 at the private clinic.",
                Some(TapbackKind::Like),
            )?]));
    let request = scan_request(
        &[chat("design-partners", 3, &["p1", "p2", "p3"])],
        &[],
        true,
        1,
        0,
    )?;
    let provider = UnavailableTestProvider;
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    let result = scan_selected_chats_with_dependencies(
        request,
        &db_path,
        ScanSelectedChatsDependencies {
            source: &source,
            provider: &provider,
            proposal_adapter: &adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())?;

    // Then
    assert_counts(&result, (1, 1, 0, 1, 0));
    assert_eq!(result.created_external_proposal_count, 1);
    assert_eq!(result.failed_external_proposal_count, 0);
    assert_eq!(adapter.created_count(), 1);
    Ok(())
}

#[test]
fn reminder_replay_stream_selects_visible_unmapped_task_reminder() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-reminder-replay-selection.sqlite")?;
    let store = Store::open(&db_path).map_err(|error| error.to_string())?;
    let candidate_id = store
        .create_candidate(CandidateDraft {
            kind: CandidateKind::TaskReminder,
            chat_guid: "design-partners".to_owned(),
            anchor_message_guid: "msg-reminder-replay-selection".to_owned(),
            title: "Daily list: 2 anchovies; 3 salmon".to_owned(),
            confidence_millis: 860,
            normalized_time: "2026-07-07T23:59:00[Asia/Seoul]".to_owned(),
            evidence_excerpt: "2 anchovies, 3 salmon".to_owned(),
            observed_at: 1_782_352_400,
        })
        .map_err(|error| error.to_string())?;
    store
        .transition_candidate(
            &candidate_id,
            CandidateState::CreatingExternal,
            "cap_selected_for_external_creation",
            1_782_352_401,
        )
        .map_err(|error| error.to_string())?;
    store
        .transition_candidate(
            &candidate_id,
            CandidateState::Visible,
            "visible_for_replay_selection",
            1_782_352_402,
        )
        .map_err(|error| error.to_string())?;

    // When
    let selected = store
        .next_replay_candidates(ReplayStream::ReminderProposals, 10)
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(selected, vec![candidate_id.clone()]);

    // Given: the Reminders mapping has been recorded after proposal creation.
    store
        .upsert_external_mapping(ExternalObjectMapping {
            candidate_id: candidate_id.clone(),
            source: ExternalSource::Reminders,
            external_object_id: "reminder-replay-selection-1".to_owned(),
            external_source_id: "reminders-list-replay-selection".to_owned(),
            mapped_at: 1_782_352_402,
        })
        .map_err(|error| error.to_string())?;

    // When
    let selected_after_mapping = store
        .next_replay_candidates(ReplayStream::ReminderProposals, 10)
        .map_err(|error| error.to_string())?;

    // Then
    assert!(selected_after_mapping.is_empty());
    Ok(())
}
