use super::super::dependencies::{CandidateProvider, CountingProvider, RecordingProposalAdapter};
use super::super::message_sqlite::{provider_route_fingerprint_count, provider_route_outcome_dump};
use super::{provider_route_request, scan_provider_route, ProviderRouteFixture};

#[test]
fn provider_route_ledger_reuses_cache_for_repeated_same_profile_request() -> Result<(), String> {
    // Given
    let fixture = ProviderRouteFixture::new()?;
    let request = provider_route_request()?;
    let provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    scan_provider_route(&fixture, request.clone(), &provider, &adapter, &recorder)?;
    scan_provider_route(&fixture, request, &provider, &adapter, &recorder)?;

    // Then
    let dump = provider_route_outcome_dump(&fixture.store_path)?;
    assert_eq!(provider.calls(), 1, "{dump}");
    assert_eq!(provider_route_fingerprint_count(&fixture.store_path)?, 1);
    assert!(dump.contains("list-reminders"), "{dump}");
    assert!(!dump.contains("2 anchovies, 3 salmon"), "{dump}");
    println!(
        "same_profile_cache_provider_calls={} fingerprint_rows={} dump={}",
        provider.calls(),
        provider_route_fingerprint_count(&fixture.store_path)?,
        dump
    );
    Ok(())
}
