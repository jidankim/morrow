use morrow_detection::{AiProvider, ProviderError, ProviderRequest, ProviderResponse};
use morrow_lib::native_bridge::{
    scan_selected_chats_with_dependencies, ScanSelectedChatsDependencies,
};
use serde_json::json;

use super::dependencies::{CountingProvider, RecordingProposalAdapter};
use super::message_sqlite::{
    create_messages_fixture, external_mapping_count, provider_route_outcome_count,
    provider_route_outcome_dump, update_provider_route_message_text,
};
use super::support::{assert_counts, chat, scan_request};

pub(super) struct CalendarScanOutcome {
    pub(super) created_titles: Vec<String>,
    pub(super) provider_calls: usize,
    pub(super) external_mappings: i64,
    pub(super) route_rows: i64,
    pub(super) ledger_dump: String,
}

pub(super) fn scan_weak_calendar_with_provider<P>(
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
    let ledger_dump = provider_route_outcome_dump(&fixture.store_path)?;

    Ok(CalendarScanOutcome {
        created_titles: adapter.created_titles(),
        provider_calls: provider.calls(),
        external_mappings,
        route_rows,
        ledger_dump,
    })
}

#[derive(Debug, Clone, Copy)]
pub(super) struct CalendarTitleProvider {
    title: &'static str,
    normalized_time: &'static str,
}

impl CalendarTitleProvider {
    pub(super) const fn new(title: &'static str, normalized_time: &'static str) -> Self {
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

pub(super) struct MessagesFixture {
    _dir: tempfile::TempDir,
    pub(super) store_path: std::path::PathBuf,
    pub(super) messages_db_path: std::path::PathBuf,
}

impl MessagesFixture {
    pub(super) fn with_text(text: &str) -> Result<Self, String> {
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

pub(super) fn messages_request(
) -> Result<morrow_lib::native_bridge::ScanSelectedChatsRequest, String> {
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
