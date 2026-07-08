use super::super::dependencies::{CandidateProvider, CountingProvider, RecordingProposalAdapter};
use super::super::message_sqlite::{
    provider_route_candidate_normalized_times, provider_route_fingerprint_count,
    provider_route_outcome_dump, set_provider_route_candidate_normalized_time,
};
use super::{
    provider_route_request, provider_route_request_with_options, scan_provider_route,
    ProviderRouteFixture,
};

#[test]
fn provider_route_ledger_records_los_angeles_timezone_reference() -> Result<(), String> {
    // Given
    let fixture = ProviderRouteFixture::new()?;
    let mut request = provider_route_request()?;
    request.reference_timezone = "America/Los_Angeles".to_owned();
    let provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    scan_provider_route(&fixture, request, &provider, &adapter, &recorder)?;

    // Then
    let dump = provider_route_outcome_dump(&fixture.store_path)?;
    assert!(
        dump.contains("2026-06-24[America/Los_Angeles]|America/Los_Angeles"),
        "{dump}"
    );
    Ok(())
}

#[test]
fn provider_route_ledger_reuses_same_local_day_reference_scope() -> Result<(), String> {
    // Given
    let fixture = ProviderRouteFixture::new()?;
    let first_request = provider_route_request_with_options(true, "Asia/Seoul", 1_782_352_400)?;
    let same_day_request = provider_route_request_with_options(true, "Asia/Seoul", 1_782_359_580)?;
    let provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    scan_provider_route(&fixture, first_request, &provider, &adapter, &recorder)?;
    scan_provider_route(&fixture, same_day_request, &provider, &adapter, &recorder)?;

    // Then
    assert_eq!(provider.calls(), 1);
    assert_eq!(provider_route_fingerprint_count(&fixture.store_path)?, 1);
    let dump = provider_route_outcome_dump(&fixture.store_path)?;
    assert!(
        dump.contains("provider-route-ledger-v2"),
        "v2 cache rows must carry the bumped contract: {dump}"
    );
    assert!(
        dump.contains("2026-06-25[Asia/Seoul]|Asia/Seoul"),
        "reference scope must be local-day plus timezone, not exact seconds: {dump}"
    );
    assert!(
        !dump.contains("2026-06-25T10:53:00[Asia/Seoul]"),
        "route ledger should not store exact reference seconds: {dump}"
    );
    println!(
        "same_day_scope_provider_calls={} fingerprint_rows={} ledger={}",
        provider.calls(),
        provider_route_fingerprint_count(&fixture.store_path)?,
        dump.trim()
    );
    Ok(())
}

#[test]
fn provider_route_ledger_cross_local_day_still_invalidates() -> Result<(), String> {
    // Given
    let fixture = ProviderRouteFixture::new()?;
    let first_request = provider_route_request_with_options(true, "Asia/Seoul", 1_782_352_400)?;
    let next_day_request = provider_route_request_with_options(true, "Asia/Seoul", 1_782_402_000)?;
    let provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    scan_provider_route(&fixture, first_request, &provider, &adapter, &recorder)?;
    scan_provider_route(&fixture, next_day_request, &provider, &adapter, &recorder)?;

    // Then
    assert_eq!(provider.calls(), 2);
    assert_eq!(provider_route_fingerprint_count(&fixture.store_path)?, 2);
    let dump = provider_route_outcome_dump(&fixture.store_path)?;
    assert!(dump.contains("2026-06-25[Asia/Seoul]|Asia/Seoul"), "{dump}");
    assert!(dump.contains("2026-06-26[Asia/Seoul]|Asia/Seoul"), "{dump}");
    println!(
        "cross_day_scope_provider_calls={} fingerprint_rows={}",
        provider.calls(),
        provider_route_fingerprint_count(&fixture.store_path)?
    );
    Ok(())
}

#[test]
fn provider_route_ledger_rejects_same_day_stale_cached_candidate() -> Result<(), String> {
    // Given
    let fixture = ProviderRouteFixture::new()?;
    let first_request = provider_route_request_with_options(true, "Asia/Seoul", 1_782_352_400)?;
    let later_same_day_request =
        provider_route_request_with_options(true, "Asia/Seoul", 1_782_384_000)?;
    let provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    scan_provider_route(&fixture, first_request, &provider, &adapter, &recorder)?;
    set_provider_route_candidate_normalized_time(
        &fixture.store_path,
        "2026-06-25T18:00:00[Asia/Seoul]",
    )?;
    scan_provider_route(
        &fixture,
        later_same_day_request,
        &provider,
        &adapter,
        &recorder,
    )?;

    // Then
    assert_eq!(provider.calls(), 2);
    assert_eq!(provider_route_fingerprint_count(&fixture.store_path)?, 1);
    let normalized_times = provider_route_candidate_normalized_times(&fixture.store_path)?;
    assert!(
        normalized_times.contains("2026-06-26T15:00:00[Asia/Seoul]"),
        "stale candidate row must be replaced by a fresh provider result: {normalized_times}"
    );
    println!(
        "stale_candidate_provider_calls={} fingerprint_rows={} normalized_times={}",
        provider.calls(),
        provider_route_fingerprint_count(&fixture.store_path)?,
        normalized_times.trim()
    );
    Ok(())
}

#[test]
fn provider_route_ledger_rejects_same_day_malformed_cached_candidate() -> Result<(), String> {
    // Given
    let fixture = ProviderRouteFixture::new()?;
    let first_request = provider_route_request_with_options(true, "Asia/Seoul", 1_782_352_400)?;
    let same_day_request = provider_route_request_with_options(true, "Asia/Seoul", 1_782_359_580)?;
    let provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    scan_provider_route(&fixture, first_request, &provider, &adapter, &recorder)?;
    set_provider_route_candidate_normalized_time(&fixture.store_path, "not a normalized time")?;
    scan_provider_route(&fixture, same_day_request, &provider, &adapter, &recorder)?;

    // Then
    assert_eq!(provider.calls(), 2);
    assert_eq!(provider_route_fingerprint_count(&fixture.store_path)?, 1);
    let normalized_times = provider_route_candidate_normalized_times(&fixture.store_path)?;
    assert!(
        normalized_times.contains("2026-06-26T15:00:00[Asia/Seoul]"),
        "malformed candidate row must be replaced by a fresh provider result: {normalized_times}"
    );
    println!(
        "malformed_candidate_provider_calls={} fingerprint_rows={} normalized_times={}",
        provider.calls(),
        provider_route_fingerprint_count(&fixture.store_path)?,
        normalized_times.trim()
    );
    Ok(())
}
