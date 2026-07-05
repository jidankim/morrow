use morrow_lib::native_bridge::{
    scan_selected_chats_with_dependencies, ScanSelectedChatsDependencies,
};
use morrow_storage::{CandidateState, Store};

use super::dependencies::{RecordingProposalAdapter, TaskReminderProvider};
use super::message_sqlite::{
    candidate_external_receipt_count, candidate_reasons, create_messages_fixture,
    drop_external_mapping_failure_trigger, external_mapping_count_for_source,
    install_external_mapping_failure_trigger,
};
use super::support::{assert_counts, candidate_state, chat, scan_request};

#[test]
fn eventkit_reminder_replay_retry_does_not_duplicate_after_storage_failure() -> Result<(), String> {
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
    let provider = TaskReminderProvider;
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
    assert_eq!(adapter.created_reminder_count(), 1);
    assert_eq!(
        external_mapping_count_for_source(&store_path, "reminders")?,
        1
    );
    assert_eq!(candidate_external_receipt_count(&store_path)?, 1);
    let store = Store::open(&store_path).map_err(|error| error.to_string())?;
    assert_eq!(candidate_state(&store, &first)?, CandidateState::Visible);
    Ok(())
}

#[test]
fn eventkit_reminder_permission_denied_records_privacy_safe_failure_state() -> Result<(), String> {
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
    let provider = TaskReminderProvider;
    let adapter = RecordingProposalAdapter::failing_reminder_permission_denied();
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
    assert_eq!(adapter.created_reminder_count(), 0);
    let store = Store::open(&store_path).map_err(|error| error.to_string())?;
    assert_eq!(candidate_state(&store, &result)?, CandidateState::Failed);
    let reasons = candidate_reasons(&store_path)?;
    assert!(
        reasons.contains("external_proposal_creation_failed: Reminders permission denied"),
        "{reasons}"
    );
    for forbidden in [
        "+15555550103",
        "iMessage;-;+15555550103",
        "Maybe meet tomorrow?",
        "Finish review of the essay",
        "beta-provider-route",
        "reminder-injected-",
        "reminders-list-injected-",
    ] {
        assert!(
            !reasons.contains(forbidden),
            "failure state leaked {forbidden}: {reasons}"
        );
    }
    Ok(())
}
