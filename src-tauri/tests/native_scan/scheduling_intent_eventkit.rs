use morrow_detection::{AiProvider, ProviderError, ProviderRequest, ProviderResponse};
use morrow_lib::native_bridge::{
    scan_selected_chats_with_dependencies, ScanSelectedChatsDependencies,
};
use serde_json::json;

use super::dependencies::{
    CandidateProvider, CountingProvider, RecordingProposalAdapter, TaskReminderProvider,
    UnavailableTestProvider,
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

struct CalendarScanOutcome {
    created_titles: Vec<String>,
    provider_calls: usize,
    external_mappings: i64,
    route_rows: i64,
}

fn scan_weak_calendar_with_provider<P>(
    message_text: &str,
    provider: &CountingProvider<P>,
) -> Result<CalendarScanOutcome, String>
where
    P: AiProvider,
{
    let fixture = MessagesFixture::with_text(message_text)?;
    let source = morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter::new(
        fixture.messages_db_path.clone(),
    );
    let request = messages_request()?;
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    let result = scan_selected_chats_with_dependencies(
        request,
        &fixture.store_path,
        ScanSelectedChatsDependencies {
            source: &source,
            provider,
            proposal_adapter: &adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())?;

    assert_counts(&result, (1, 1, 0, 1, 0));
    assert_eq!(result.created_external_proposal_count, 1);
    assert_eq!(result.failed_external_proposal_count, 0);
    assert_eq!(adapter.created_count(), 1);
    let external_mappings = external_mapping_count(&fixture.store_path)?;
    let route_rows = provider_route_outcome_count(&fixture.store_path)?;

    Ok(CalendarScanOutcome {
        created_titles: adapter.created_titles(),
        provider_calls: provider.calls(),
        external_mappings,
        route_rows,
    })
}

#[derive(Debug, Clone, Copy)]
struct CalendarTitleProvider {
    title: &'static str,
    normalized_time: &'static str,
}

impl CalendarTitleProvider {
    const fn new(title: &'static str, normalized_time: &'static str) -> Self {
        Self {
            title,
            normalized_time,
        }
    }
}

impl AiProvider for CalendarTitleProvider {
    fn extract(&self, _request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        let response = json!({
            "kind": "calendar_event",
            "title": self.title,
            "confidence_millis": 860,
            "normalized_time": self.normalized_time,
            "anchor_message_guid": "beta-provider-route",
            "evidence_message_guids": ["beta-provider-route"],
        });
        Ok(ProviderResponse::new(&response.to_string()))
    }
}

mod scheduling_intent_regression {
    use super::*;

    const COFFEE_SYNC_MESSAGE: &str =
        "Morrow QA live receipt test: coffee sync on 2026-07-17 at 9:30 AM for 30 minutes";
    const COFFEE_SYNC_TIME: &str = "2026-07-17T09:30:00[Asia/Seoul]";

    #[test]
    fn production_scan_creates_calendar_proposal_for_coffee_sync_meridiem_message(
    ) -> Result<(), String> {
        // Given
        let provider =
            CountingProvider::new(CalendarTitleProvider::new("coffee sync", COFFEE_SYNC_TIME));

        // When
        let outcome = scan_weak_calendar_with_provider(COFFEE_SYNC_MESSAGE, &provider)?;

        // Then
        assert_eq!(outcome.provider_calls, 1);
        assert_eq!(outcome.created_titles, ["coffee sync"]);
        assert_eq!(outcome.external_mappings, 1);
        assert_eq!(outcome.route_rows, 1);
        println!(
            "coffee_sync_provider_route_native provider_calls={} external_mappings={} route_rows={}",
            outcome.provider_calls, outcome.external_mappings, outcome.route_rows
        );
        Ok(())
    }

    #[test]
    fn production_scan_falls_back_to_calendar_proposal_when_coffee_sync_provider_unavailable(
    ) -> Result<(), String> {
        // Given
        let provider = CountingProvider::new(UnavailableTestProvider);

        // When
        let outcome = scan_weak_calendar_with_provider(COFFEE_SYNC_MESSAGE, &provider)?;

        // Then
        assert_eq!(outcome.provider_calls, 1);
        assert_eq!(outcome.created_titles, ["coffee sync"]);
        assert_eq!(outcome.external_mappings, 1);
        assert_eq!(outcome.route_rows, 0);
        println!(
            "coffee_sync_provider_unavailable_fallback provider_calls={} external_mappings={} route_rows={}",
            outcome.provider_calls, outcome.external_mappings, outcome.route_rows
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
