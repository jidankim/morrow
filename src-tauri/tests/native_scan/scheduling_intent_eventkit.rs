use morrow_lib::native_bridge::{
    scan_selected_chats_with_dependencies, ScanSelectedChatsDependencies,
};

use super::dependencies::{
    CandidateProvider, CountingProvider, RecordingProposalAdapter, TaskReminderProvider,
};
use super::message_sqlite::{
    create_messages_fixture, external_mapping_count, external_mapping_count_for_source,
    provider_route_outcome_count, provider_route_outcome_dump, update_provider_route_message_text,
};
use super::support::{assert_counts, chat, scan_request};

#[test]
fn scheduling_intent_native_scan_routes_weak_calendar_to_provider_proposal() -> Result<(), String> {
    // Given
    let fixture = MessagesFixture::with_text("Catch up Friday afternoon?")?;
    let source = morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter::new(
        fixture.messages_db_path.clone(),
    );
    let request = messages_request()?;
    let provider = CandidateProvider;
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    let result = scan_selected_chats_with_dependencies(
        request,
        &fixture.store_path,
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
    assert_eq!(adapter.created_titles(), ["Provider supplied title"]);
    assert_eq!(adapter.created_count(), 1);
    assert_eq!(external_mapping_count(&fixture.store_path)?, 1);
    assert_eq!(provider_route_outcome_count(&fixture.store_path)?, 1);
    let dump = provider_route_outcome_dump(&fixture.store_path)?;
    assert!(
        dump.contains("|parser_provider_route_weak_calendar|candidate|"),
        "{dump}"
    );
    println!(
        "weak_calendar_provider_proposal provider_calls=1 external_mappings={} route_rows={} dump={}",
        external_mapping_count(&fixture.store_path)?,
        provider_route_outcome_count(&fixture.store_path)?,
        dump
    );
    Ok(())
}

#[test]
fn scheduling_intent_native_scan_routes_weak_task_to_reminders_proposal() -> Result<(), String> {
    // Given
    let fixture = MessagesFixture::with_text("Follow up by July 25, 2026.")?;
    let source = morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter::new(
        fixture.messages_db_path.clone(),
    );
    let request = messages_request()?;
    let provider = TaskReminderProvider;
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    let result = scan_selected_chats_with_dependencies(
        request,
        &fixture.store_path,
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
    assert_eq!(adapter.created_titles(), ["Finish review of the essay"]);
    assert_eq!(adapter.created_reminder_count(), 1);
    assert_eq!(
        external_mapping_count_for_source(&fixture.store_path, "reminders")?,
        1
    );
    assert_eq!(provider_route_outcome_count(&fixture.store_path)?, 1);
    let dump = provider_route_outcome_dump(&fixture.store_path)?;
    assert!(
        dump.contains("|parser_provider_route_task_deadline|candidate|"),
        "{dump}"
    );
    println!(
        "weak_task_reminders_proposal provider_calls=1 reminder_mappings={} route_rows={} dump={}",
        external_mapping_count_for_source(&fixture.store_path, "reminders")?,
        provider_route_outcome_count(&fixture.store_path)?,
        dump
    );
    Ok(())
}

mod scheduling_intent_regression {
    use super::*;

    #[test]
    fn production_scan_creates_calendar_proposal_for_coffee_sync_meridiem_message(
    ) -> Result<(), String> {
        // Given
        let fixture = MessagesFixture::with_text(
            "Morrow QA live receipt test: coffee sync on 2026-07-17 at 9:30 AM for 30 minutes",
        )?;
        let source = morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter::new(
            fixture.messages_db_path.clone(),
        );
        let request = messages_request()?;
        let provider = CountingProvider::new(CandidateProvider);
        let adapter = RecordingProposalAdapter::default();
        let recorder = morrow_diagnostics::NoopTraceRecorder;

        // When
        let result = scan_selected_chats_with_dependencies(
            request,
            &fixture.store_path,
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
        assert_eq!(provider.calls(), 0);
        assert_eq!(result.created_external_proposal_count, 1);
        assert_eq!(adapter.created_count(), 1);
        assert_eq!(external_mapping_count(&fixture.store_path)?, 1);
        assert_eq!(provider_route_outcome_count(&fixture.store_path)?, 0);
        println!(
            "coffee_sync_deterministic_native provider_calls={} external_mappings={} route_rows={}",
            provider.calls(),
            external_mapping_count(&fixture.store_path)?,
            provider_route_outcome_count(&fixture.store_path)?
        );
        Ok(())
    }
}

struct MessagesFixture {
    _dir: tempfile::TempDir,
    store_path: std::path::PathBuf,
    messages_db_path: std::path::PathBuf,
}

impl MessagesFixture {
    fn with_text(text: &str) -> Result<Self, String> {
        let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let store_path = dir.path().join("morrow.sqlite");
        let messages_db_path = dir.path().join("chat.db");
        create_messages_fixture(&messages_db_path)?;
        update_provider_route_message_text(&messages_db_path, text)?;
        Ok(Self {
            _dir: dir,
            store_path,
            messages_db_path,
        })
    }
}

fn messages_request() -> Result<morrow_lib::native_bridge::ScanSelectedChatsRequest, String> {
    scan_request(
        &[chat(
            "iMessage;-;+15555550103",
            1,
            &["messages-participant-6044b729eea9fa126e78d421e4a41ac8"],
        )],
        &[],
        true,
        1,
        0,
    )
}
