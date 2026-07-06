use morrow_lib::native_bridge::{
    scan_selected_chats_with_dependencies, FakeNativeBridge, NativeBridgeState,
    ScanSelectedChatsDependencies,
};
use morrow_messages::TapbackKind;
use morrow_storage::{CandidateState, Store};

use super::dependencies::{CandidateProvider, RecordingProposalAdapter, UnavailableTestProvider};
use super::message_sqlite::{
    candidate_external_receipt_count, candidate_reasons, create_messages_fixture,
    drop_external_mapping_failure_trigger, external_mapping_count,
    install_external_mapping_failure_trigger,
};
use super::support::{
    assert_counts, batch, candidate_state, chat, raw_chat, scan_request, temp_db,
};

#[test]
fn production_scan_reads_messages_before_provider_unavailable_boundary() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let store_path = dir.path().join("morrow.sqlite");
    let messages_db_path = dir.path().join("chat.db");
    create_messages_fixture(&messages_db_path)?;
    let request = scan_request(
        &[chat(
            "messages-chat-8b96e568c027a42d3b5c9e6e7710201f",
            1,
            &["messages-participant-6044b729eea9fa126e78d421e4a41ac8"],
        )],
        &[],
        true,
        1,
        0,
    )?;

    // When
    let result = NativeBridgeState::default()
        .scan_selected_chats_at(request, &store_path, &messages_db_path)
        .map_err(|error| error.to_string())?;

    // Then
    assert_counts(&result, (0, 0, 1, 0, 0));
    Ok(())
}

#[test]
fn production_scan_uses_provider_and_proposal_adapter_dependencies() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let store_path = dir.path().join("morrow.sqlite");
    let messages_db_path = dir.path().join("chat.db");
    create_messages_fixture(&messages_db_path)?;
    let source =
        morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter::new(messages_db_path);
    let request = scan_request(
        &[chat(
            "iMessage;-;+15555550103",
            1,
            &["messages-participant-6044b729eea9fa126e78d421e4a41ac8"],
        )],
        &[],
        true,
        1,
        0,
    )?;
    let provider = CandidateProvider;
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    let result = scan_selected_chats_with_dependencies(
        request,
        &store_path,
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
    assert_eq!(adapter.created_titles(), ["Provider supplied title"]);
    let store = Store::open(&store_path).map_err(|error| error.to_string())?;
    assert_eq!(candidate_state(&store, &result)?, CandidateState::Visible);
    Ok(())
}

#[test]
fn eventkit_replay_failure_records_privacy_safe_failure_state() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let store_path = dir.path().join("morrow.sqlite");
    let messages_db_path = dir.path().join("chat.db");
    create_messages_fixture(&messages_db_path)?;
    let source =
        morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter::new(messages_db_path);
    let request = scan_request(
        &[chat(
            "iMessage;-;+15555550103",
            1,
            &["messages-participant-6044b729eea9fa126e78d421e4a41ac8"],
        )],
        &[],
        true,
        1,
        0,
    )?;
    let provider = CandidateProvider;
    let adapter = RecordingProposalAdapter::failing_calendar();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    let result = scan_selected_chats_with_dependencies(
        request,
        &store_path,
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
    assert_eq!(result.created_external_proposal_count, 0);
    assert_eq!(result.failed_external_proposal_count, 1);
    let store = Store::open(&store_path).map_err(|error| error.to_string())?;
    assert_eq!(candidate_state(&store, &result)?, CandidateState::Failed);
    let reasons = candidate_reasons(&store_path)?;
    assert!(
        reasons.contains("external_proposal_creation_failed"),
        "{reasons}"
    );
    for forbidden in [
        "+15555550103",
        "Maybe meet tomorrow?",
        "beta-provider-route",
    ] {
        assert!(
            !reasons.contains(forbidden),
            "failure state leaked {forbidden}: {reasons}"
        );
    }
    Ok(())
}

#[test]
fn eventkit_replay_retry_does_not_duplicate_after_storage_failure() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let store_path = dir.path().join("morrow.sqlite");
    let messages_db_path = dir.path().join("chat.db");
    create_messages_fixture(&messages_db_path)?;
    let _store = Store::open(&store_path).map_err(|error| error.to_string())?;
    install_external_mapping_failure_trigger(&store_path)?;
    let source =
        morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter::new(messages_db_path);
    let request = scan_request(
        &[chat(
            "iMessage;-;+15555550103",
            1,
            &["messages-participant-6044b729eea9fa126e78d421e4a41ac8"],
        )],
        &[],
        true,
        1,
        0,
    )?;
    let provider = CandidateProvider;
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    let first = scan_selected_chats_with_dependencies(
        request.clone(),
        &store_path,
        ScanSelectedChatsDependencies {
            source: &source,
            provider: &provider,
            proposal_adapter: &adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())?;
    drop_external_mapping_failure_trigger(&store_path)?;
    let second = scan_selected_chats_with_dependencies(
        request,
        &store_path,
        ScanSelectedChatsDependencies {
            source: &source,
            provider: &provider,
            proposal_adapter: &adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())?;

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
fn injected_unavailable_provider_records_quiet_failure_without_candidates() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-injected-unavailable.sqlite")?;
    let source =
        FakeNativeBridge::with_morrow_store_path(db_path.clone()).with_messages(batch(vec![
            raw_chat(
                "design-partners",
                "msg-provider-route",
                "Maybe meet tomorrow?",
                Some(TapbackKind::Like),
            )?,
        ]));
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
    assert_counts(&result, (0, 0, 1, 0, 0));
    assert_eq!(result.created_external_proposal_count, 0);
    assert_eq!(adapter.created_titles().len(), 0);
    Ok(())
}
