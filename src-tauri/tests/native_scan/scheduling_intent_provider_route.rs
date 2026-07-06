use morrow_lib::native_bridge::{
    scan_selected_chats_with_dependencies, ScanSelectedChatsDependencies,
};

use super::dependencies::{CandidateProvider, CountingProvider, RecordingProposalAdapter};
use super::message_sqlite::{
    external_mapping_count, provider_route_outcome_count, provider_route_outcome_dump,
};
use super::provider_route_ledger::{provider_route_request, ProviderRouteFixture};
use super::support::assert_counts;

#[test]
fn scheduling_intent_native_scan_cache_hit_skips_second_provider_call() -> Result<(), String> {
    // Given
    let fixture = ProviderRouteFixture::with_text("Catch up Friday afternoon?")?;
    let request = provider_route_request()?;
    let provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    let first = scan_selected_chats_with_dependencies(
        request.clone(),
        &fixture.store_path,
        ScanSelectedChatsDependencies {
            source: &fixture.source,
            provider: &provider,
            proposal_adapter: &adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())?;
    let first_provider_calls = provider.calls();
    let second = scan_selected_chats_with_dependencies(
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
    let second_provider_calls = provider.calls();

    // Then
    assert_eq!(first_provider_calls, 1);
    assert_eq!(second_provider_calls, 1);
    assert_counts(&first, (1, 1, 0, 1, 0));
    assert_counts(&second, (0, 0, 0, 0, 0));
    assert_eq!(adapter.created_count(), 1);
    assert_eq!(provider_route_outcome_count(&fixture.store_path)?, 1);
    let dump = provider_route_outcome_dump(&fixture.store_path)?;
    assert!(
        dump.contains("|parser_provider_route_weak_calendar|candidate|"),
        "{dump}"
    );
    println!(
        "weak_calendar_cache_hit first_provider_calls={first_provider_calls} second_provider_calls={second_provider_calls} route_rows={} dump={}",
        provider_route_outcome_count(&fixture.store_path)?,
        dump
    );
    Ok(())
}

#[test]
fn scheduling_intent_native_scan_failed_replay_does_not_write_provider_route_row(
) -> Result<(), String> {
    // Given
    let fixture = ProviderRouteFixture::with_text("Catch up Friday afternoon?")?;
    let request = provider_route_request()?;
    let provider = CountingProvider::new(CandidateProvider);
    let failing_adapter = RecordingProposalAdapter::failing_calendar();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    let failed = scan_selected_chats_with_dependencies(
        request,
        &fixture.store_path,
        ScanSelectedChatsDependencies {
            source: &fixture.source,
            provider: &provider,
            proposal_adapter: &failing_adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())?;
    let rows_after_failure = provider_route_outcome_count(&fixture.store_path)?;

    // Then
    assert_eq!(failed.created_external_proposal_count, 0);
    assert_eq!(failed.failed_external_proposal_count, 1);
    assert_eq!(provider.calls(), 1);
    assert_eq!(failing_adapter.created_count(), 0);
    assert_eq!(external_mapping_count(&fixture.store_path)?, 0);
    assert_eq!(rows_after_failure, 0);
    println!(
        "weak_calendar_failed_replay provider_calls={} external_mappings={} route_rows={rows_after_failure}",
        provider.calls(),
        external_mapping_count(&fixture.store_path)?
    );
    Ok(())
}
