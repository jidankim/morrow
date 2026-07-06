use morrow_lib::native_bridge::{
    scan_selected_chats_with_dependencies, ScanSelectedChatsDependencies,
};
use morrow_storage::{CandidateState, Store};

use super::dependencies::{CountingProvider, RecordingProposalAdapter, UnavailableTestProvider};
use super::message_sqlite::{
    create_messages_fixture, external_mapping_count, update_provider_route_message_text,
};
use super::support::{assert_counts, candidate_state, chat, scan_request};

const COFFEE_SYNC: &str =
    "Morrow QA live receipt test: coffee sync on 2026-07-17 at 9:30 AM for 30 minutes";

#[test]
fn production_scan_creates_calendar_proposal_for_coffee_sync_meridiem_message() -> Result<(), String>
{
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let store_path = dir.path().join("morrow.sqlite");
    let messages_db_path = dir.path().join("chat.db");
    create_messages_fixture(&messages_db_path)?;
    update_provider_route_message_text(&messages_db_path, COFFEE_SYNC)?;
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
    let provider = CountingProvider::new(UnavailableTestProvider);
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
    assert_eq!(provider.calls(), 0);
    assert_counts(&result, (1, 1, 0, 1, 0));
    assert_eq!(result.created_external_proposal_count, 1);
    assert_eq!(result.failed_external_proposal_count, 0);
    assert_eq!(adapter.created_titles(), ["Messages event candidate"]);
    assert_eq!(external_mapping_count(&store_path)?, 1);
    let store = Store::open(&store_path).map_err(|error| error.to_string())?;
    assert_eq!(candidate_state(&store, &result)?, CandidateState::Visible);
    Ok(())
}
