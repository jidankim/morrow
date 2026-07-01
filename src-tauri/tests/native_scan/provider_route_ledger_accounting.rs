use morrow_diagnostics::{TraceComponent, TraceOperation, TraceOutcome, TraceRecord};
use morrow_lib::native_bridge::{
    scan_selected_chats_with_dependencies, ScanSelectedChatsDependencies, ScanSelectedChatsRequest,
};
use morrow_storage::{CandidateState, Store};

use super::dependencies::{
    CandidateProvider, CountingProvider, InvalidJsonProvider, RecordingProposalAdapter,
};
use super::message_sqlite::{
    candidate_external_receipt_count, create_messages_fixture, external_mapping_count,
    provider_route_outcome_count,
};
use super::support::{
    assert_counts, candidate_state, chat, query_sqlite, scan_request_with_feedback_text,
};
use super::trace_support::RecordingTraceRecorder;

#[test]
fn provider_route_ledger_repeated_scan_has_zero_new_counts() -> Result<(), String> {
    // Given
    let fixture = ProviderRouteFixture::new()?;
    let request = provider_route_feedback_request()?;
    let provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = RecordingTraceRecorder::default();

    // When
    let first = scan_provider_route(&fixture, request.clone(), &provider, &adapter, &recorder)?;
    let persisted_after_first = scan_accounting_counts(&fixture.store_path)?;
    let first_trace_count = recorder.records()?.len();
    let second = scan_provider_route(&fixture, request, &provider, &adapter, &recorder)?;
    let persisted_after_second = scan_accounting_counts(&fixture.store_path)?;
    let records = recorder.records()?;

    // Then
    assert_counts(&first, (1, 1, 0, 1, 0));
    assert_eq!(first.created_external_proposal_count, 1);
    assert_counts(&second, (0, 0, 0, 0, 0));
    assert_eq!(second.created_external_proposal_count, 0);
    assert_eq!(second.failed_external_proposal_count, 0);
    assert_eq!(second.feedback_label_count, first.feedback_label_count);
    assert_eq!(second.feature_snapshot_count, first.feature_snapshot_count);
    assert_eq!(provider.calls(), 1);
    assert_eq!(adapter.created_count(), 1);
    assert_eq!(persisted_after_second, persisted_after_first);
    assert_eq!(provider_route_outcome_count(&fixture.store_path)?, 1);
    assert_cache_hit_trace(&records[first_trace_count..]);
    println!(
        "second_scan_counts candidates={} quiet={} external={} provider_calls={}",
        second.created_candidate_count,
        second.quiet_log_count,
        second.created_external_proposal_count,
        provider.calls()
    );
    Ok(())
}

#[test]
fn provider_route_ledger_existing_calendar_mapping_not_replayed() -> Result<(), String> {
    // Given
    let fixture = ProviderRouteFixture::new()?;
    let request = provider_route_feedback_request()?;
    let provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = RecordingTraceRecorder::default();

    // When
    let first = scan_provider_route(&fixture, request.clone(), &provider, &adapter, &recorder)?;
    let first_trace_count = recorder.records()?.len();
    let second = scan_provider_route(&fixture, request, &provider, &adapter, &recorder)?;
    let records = recorder.records()?;
    let store = Store::open(&fixture.store_path).map_err(|error| error.to_string())?;

    // Then
    assert_eq!(candidate_state(&store, &first)?, CandidateState::Visible);
    assert_counts(&second, (0, 0, 0, 0, 0));
    assert_eq!(second.created_external_proposal_count, 0);
    assert_eq!(second.failed_external_proposal_count, 0);
    assert_eq!(adapter.created_count(), 1);
    assert_eq!(external_mapping_count(&fixture.store_path)?, 1);
    assert_eq!(candidate_external_receipt_count(&fixture.store_path)?, 1);
    assert_eq!(provider.calls(), 1);
    assert_cache_hit_trace(&records[first_trace_count..]);
    println!(
        "calendar_replay_created={} mappings={} receipts={}",
        adapter.created_count(),
        external_mapping_count(&fixture.store_path)?,
        candidate_external_receipt_count(&fixture.store_path)?
    );
    Ok(())
}

#[test]
fn provider_route_ledger_stable_quiet_rejection_cache_hit_has_zero_new_counts() -> Result<(), String>
{
    // Given
    let fixture = ProviderRouteFixture::new()?;
    let request = provider_route_feedback_request()?;
    let provider = CountingProvider::new(InvalidJsonProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = RecordingTraceRecorder::default();

    // When
    let first = scan_provider_route(&fixture, request.clone(), &provider, &adapter, &recorder)?;
    let persisted_after_first = scan_accounting_counts(&fixture.store_path)?;
    let first_trace_count = recorder.records()?.len();
    let second = scan_provider_route(&fixture, request, &provider, &adapter, &recorder)?;
    let persisted_after_second = scan_accounting_counts(&fixture.store_path)?;
    let records = recorder.records()?;

    // Then
    assert_counts(&first, (0, 0, 1, 0, 0));
    assert_counts(&second, (0, 0, 0, 0, 0));
    assert_eq!(second.created_external_proposal_count, 0);
    assert_eq!(second.failed_external_proposal_count, 0);
    assert_eq!(second.feedback_label_count, first.feedback_label_count);
    assert_eq!(second.feature_snapshot_count, first.feature_snapshot_count);
    assert_eq!(provider.calls(), 1);
    assert_eq!(adapter.created_count(), 0);
    assert_eq!(persisted_after_second, persisted_after_first);
    assert_eq!(provider_route_outcome_count(&fixture.store_path)?, 1);
    assert_cache_hit_trace(&records[first_trace_count..]);
    println!(
        "quiet_cache_hit_counts quiet={} provider_calls={} provider_route_rows={}",
        second.quiet_log_count,
        provider.calls(),
        provider_route_outcome_count(&fixture.store_path)?
    );
    Ok(())
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

fn scan_accounting_counts(db_path: &std::path::Path) -> Result<String, String> {
    query_sqlite(
        db_path,
        "SELECT
            (SELECT COUNT(*) FROM candidates) || '|' ||
            (SELECT COUNT(*) FROM quiet_logs) || '|' ||
            (SELECT COUNT(*) FROM feedback_events) || '|' ||
            (SELECT COUNT(*) FROM labels) || '|' ||
            (SELECT COUNT(*) FROM feature_snapshots) || '|' ||
            (SELECT COUNT(*) FROM external_object_mappings) || '|' ||
            (SELECT COUNT(*) FROM provider_route_outcomes);",
    )
}

fn assert_cache_hit_trace(records: &[TraceRecord]) {
    assert!(
        records.iter().any(|record| {
            record.span.component == TraceComponent::Provider
                && record.span.operation == TraceOperation::ProviderRoute
                && record.span.reason_code.as_deref() == Some("provider_route_cache_hit")
                && record.span.outcome == TraceOutcome::Noop
        }),
        "cache-hit provider breadcrumb missing: {records:?}"
    );
    assert!(
        records.iter().any(|record| {
            record.span.component == TraceComponent::Outcome
                && record.span.operation == TraceOperation::OutcomeMaterialized
                && record.span.reason_code.as_deref() == Some("provider_route_cache_hit")
                && record.span.outcome == TraceOutcome::Noop
        }),
        "cached provider route outcome trace missing: {records:?}"
    );
}
