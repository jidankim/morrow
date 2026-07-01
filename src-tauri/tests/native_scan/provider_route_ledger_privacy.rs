use morrow_lib::native_bridge::{
    scan_selected_chats_with_dependencies, ScanSelectedChatsDependencies, ScanSelectedChatsRequest,
};

use super::dependencies::{
    CandidateProvider, CountingProvider, RecordingProposalAdapter, UnavailableTestProvider,
};
use super::message_sqlite::{
    create_messages_fixture, provider_route_outcome_count, provider_route_outcome_dump,
};
use super::support::{assert_counts, chat, scan_request, scan_request_with_feedback_text};
use super::trace_support::{assert_trace_records_hide_raw_native_content, RecordingTraceRecorder};

#[test]
fn provider_route_ledger_privacy() -> Result<(), String> {
    // Given
    let fixture = ProviderRouteFixture::new()?;
    let provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    scan_provider_route(
        &fixture,
        provider_route_request()?,
        &provider,
        &adapter,
        &recorder,
    )?;

    // Then
    let dump = provider_route_outcome_dump(&fixture.store_path)?;
    println!("provider_route_ledger_privacy_dump={dump}");
    assert_eq!(provider.calls(), 1);
    assert_eq!(provider_route_outcome_count(&fixture.store_path)?, 1);
    for forbidden in [
        "+15555550103",
        "Maybe meet tomorrow?",
        "{\"kind\"",
        "From:",
        "To:",
        "Provider supplied title",
    ] {
        assert!(
            !dump.contains(forbidden),
            "provider route ledger leaked forbidden substring {forbidden}: {dump}"
        );
    }
    Ok(())
}

#[test]
fn provider_route_ledger_unavailable_is_retryable() -> Result<(), String> {
    // Given
    let fixture = ProviderRouteFixture::new()?;
    let request = provider_route_request()?;
    let provider = CountingProvider::new(UnavailableTestProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    let first = scan_provider_route(&fixture, request.clone(), &provider, &adapter, &recorder)?;
    let second = scan_provider_route(&fixture, request, &provider, &adapter, &recorder)?;

    // Then
    assert_counts(&first, (0, 0, 1, 0, 0));
    assert_counts(&second, (0, 0, 1, 0, 0));
    assert_eq!(provider.calls(), 2);
    assert_eq!(provider_route_outcome_count(&fixture.store_path)?, 0);
    println!(
        "unavailable_provider_calls={} provider_route_rows=0",
        provider.calls()
    );
    Ok(())
}

#[test]
fn provider_route_ledger_cache_hit_records_trace_without_feedback_duplicates() -> Result<(), String>
{
    // Given
    let fixture = ProviderRouteFixture::new()?;
    let request = provider_route_feedback_request()?;
    let provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = RecordingTraceRecorder::default();

    // When
    let first = scan_provider_route(&fixture, request.clone(), &provider, &adapter, &recorder)?;
    let first_feedback_counts = feedback_counts(&fixture.store_path)?;
    let first_trace_count = recorder.records()?.len();
    let second = scan_provider_route(&fixture, request, &provider, &adapter, &recorder)?;
    let second_feedback_counts = feedback_counts(&fixture.store_path)?;
    let records = recorder.records()?;
    let cache_hit_records = &records[first_trace_count..];
    let cache_hit_trace =
        serde_json::to_string(cache_hit_records).map_err(|error| error.to_string())?;

    // Then
    assert_counts(&first, (1, 1, 0, 1, 0));
    assert_counts(&second, (0, 0, 0, 0, 0));
    assert_eq!(provider.calls(), 1);
    assert_eq!(adapter.created_count(), 1);
    assert_eq!(first_feedback_counts, second_feedback_counts);
    assert!(cache_hit_trace.contains("provider_route_cache_hit"));
    assert!(!cache_hit_trace.contains("provider_extract_success"));
    assert_trace_records_hide_raw_native_content(
        cache_hit_records,
        &[
            "+15555550103",
            "Maybe meet tomorrow?",
            "Provider supplied title",
        ],
    )
}

struct ProviderRouteFixture {
    _dir: tempfile::TempDir,
    store_path: std::path::PathBuf,
    source: morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter,
}

impl ProviderRouteFixture {
    fn new() -> Result<Self, String> {
        let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let store_path = dir.path().join("morrow.sqlite");
        let messages_db_path = dir.path().join("chat.db");
        create_messages_fixture(&messages_db_path)?;
        Ok(Self {
            source: morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter::new(
                messages_db_path,
            ),
            _dir: dir,
            store_path,
        })
    }
}

fn provider_route_request() -> Result<ScanSelectedChatsRequest, String> {
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

fn provider_route_feedback_request() -> Result<ScanSelectedChatsRequest, String> {
    scan_request_with_feedback_text(
        &[chat(
            "iMessage;-;+15555550103",
            1,
            &["messages-participant-6044b729eea9fa126e78d421e4a41ac8"],
        )],
        &[],
        true,
        true,
        1,
        0,
    )
}

fn scan_provider_route<P, R>(
    fixture: &ProviderRouteFixture,
    request: ScanSelectedChatsRequest,
    provider: &P,
    adapter: &RecordingProposalAdapter,
    recorder: &R,
) -> Result<morrow_lib::native_bridge::ScanSelectedChatsResult, String>
where
    P: morrow_detection::AiProvider,
    R: morrow_diagnostics::TraceRecorder + ?Sized,
{
    scan_selected_chats_with_dependencies(
        request,
        &fixture.store_path,
        ScanSelectedChatsDependencies {
            source: &fixture.source,
            provider,
            proposal_adapter: adapter,
            trace_recorder: recorder,
        },
    )
    .map_err(|error| error.to_string())
}

fn feedback_counts(db_path: &std::path::Path) -> Result<String, String> {
    super::support::query_sqlite(
        db_path,
        "SELECT
            (SELECT COUNT(*) FROM candidates) || '|' ||
            (SELECT COUNT(*) FROM quiet_logs) || '|' ||
            (SELECT COUNT(*) FROM feature_snapshots) || '|' ||
            (SELECT COUNT(*) FROM feedback_events) || '|' ||
            (SELECT COUNT(*) FROM labels) || '|' ||
            (SELECT COUNT(*) FROM external_object_mappings);",
    )
}
