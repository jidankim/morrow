use std::cell::Cell;

use morrow_detection::{AiProvider, ProviderError, ProviderRequest, ProviderResponse};
use morrow_lib::native_bridge::{
    scan_selected_chats_with_dependencies, ScanSelectedChatsDependencies,
};

use super::dependencies::RecordingProposalAdapter;
use super::message_sqlite::{
    create_messages_fixture, external_mapping_count, insert_provider_route_companion_message,
    provider_route_outcome_count, update_provider_route_message_text,
};
use super::support::{assert_counts, chat, scan_request};

#[test]
fn provider_route_same_scan_duplicate_fingerprint_calls_provider_once() -> Result<(), String> {
    // Given
    let fixture = ProviderRouteSameScanFixture::new()?;
    insert_provider_route_companion_message(&fixture.messages_db_path, "Maybe meet tomorrow?")?;
    let request = provider_route_request()?;
    let provider = SequentialRouteProvider::default();
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    let result = scan_selected_chats_with_dependencies(
        request,
        &fixture.store_path,
        ScanSelectedChatsDependencies {
            source: &fixture.source,
            provider: &provider,
            proposal_adapter: &adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())?;
    let route_rows = provider_route_outcome_count(&fixture.store_path)?;
    let mapping_rows = external_mapping_count(&fixture.store_path)?;

    // Then
    assert_eq!(provider.calls(), 1);
    assert_counts(&result, (1, 1, 0, 1, 0));
    assert_eq!(adapter.created_count(), 1);
    assert_eq!(result.created_external_proposal_count, 1);
    assert_eq!(result.failed_external_proposal_count, 0);
    assert_eq!(route_rows, 1);
    assert_eq!(mapping_rows, 1);
    println!(
        "same_scan_native provider_calls={} provider_route_rows={} external_mapping_rows={}",
        provider.calls(),
        route_rows,
        mapping_rows
    );
    Ok(())
}

struct ProviderRouteSameScanFixture {
    _dir: tempfile::TempDir,
    store_path: std::path::PathBuf,
    messages_db_path: std::path::PathBuf,
    source: morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter,
}

impl ProviderRouteSameScanFixture {
    fn new() -> Result<Self, String> {
        let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let store_path = dir.path().join("morrow.sqlite");
        let messages_db_path = dir.path().join("chat.db");
        create_messages_fixture(&messages_db_path)?;
        update_provider_route_message_text(&messages_db_path, "Maybe meet tomorrow?")?;
        Ok(Self {
            source: morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter::new(
                messages_db_path.clone(),
            ),
            _dir: dir,
            store_path,
            messages_db_path,
        })
    }
}

#[derive(Default)]
struct SequentialRouteProvider {
    calls: Cell<usize>,
}

impl SequentialRouteProvider {
    fn calls(&self) -> usize {
        self.calls.get()
    }
}

impl AiProvider for SequentialRouteProvider {
    fn extract(&self, _request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        let calls = self.calls.get();
        self.calls.set(calls + 1);
        let anchor = match calls {
            0 => "alpha-provider-context",
            _ => "beta-provider-route",
        };
        Ok(ProviderResponse::new(&format!(
            "{{\"kind\":\"calendar_event\",\"title\":\"Provider supplied title\",\
             \"confidence_millis\":800,\
             \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
             \"anchor_message_guid\":\"{anchor}\",\
             \"evidence_message_guids\":[\"{anchor}\"]}}"
        )))
    }
}

fn provider_route_request() -> Result<morrow_lib::native_bridge::ScanSelectedChatsRequest, String> {
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
