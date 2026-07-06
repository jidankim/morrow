use super::dependencies::{CandidateProvider, CountingProvider, RecordingProposalAdapter};
use super::message_sqlite::{
    provider_route_fingerprint_count, provider_route_outcome_count, provider_route_outcome_dump,
    update_provider_route_message_text,
};
use super::provider_route_support::{
    assert_provider_route_dump_hides, provider_route_request, provider_route_request_with_options,
    scan_provider_route, ProviderRouteFixture,
};

#[path = "provider_route_ledger_invalidation/profile_change.rs"]
mod profile_change;
#[path = "provider_route_ledger_invalidation/version_mismatch.rs"]
mod version_mismatch;

#[test]
fn provider_route_ledger_invalidation() -> Result<(), String> {
    // Given
    let fixture = ProviderRouteFixture::new()?;
    let request = provider_route_request()?;
    let provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    scan_provider_route(&fixture, request.clone(), &provider, &adapter, &recorder)?;
    scan_provider_route(&fixture, request.clone(), &provider, &adapter, &recorder)?;

    // Then
    assert_eq!(provider.calls(), 1);
    assert_eq!(provider_route_outcome_count(&fixture.store_path)?, 1);

    // When
    update_provider_route_message_text(
        &fixture.messages_db_path,
        "Maybe meet the project team tomorrow?",
    )?;
    scan_provider_route(&fixture, request, &provider, &adapter, &recorder)?;

    // Then
    assert_eq!(provider.calls(), 2);
    assert_eq!(provider_route_fingerprint_count(&fixture.store_path)?, 2);

    // When
    let hidden_excerpt_request =
        provider_route_request_with_options(false, "Asia/Seoul", 1_782_352_400)?;
    scan_provider_route(
        &fixture,
        hidden_excerpt_request.clone(),
        &provider,
        &adapter,
        &recorder,
    )?;
    scan_provider_route(
        &fixture,
        hidden_excerpt_request,
        &provider,
        &adapter,
        &recorder,
    )?;

    // Then
    assert_eq!(provider.calls(), 3);
    assert_eq!(provider_route_fingerprint_count(&fixture.store_path)?, 3);

    // When
    let utc_reference_request = provider_route_request_with_options(false, "UTC", 1_782_352_400)?;
    scan_provider_route(
        &fixture,
        utc_reference_request.clone(),
        &provider,
        &adapter,
        &recorder,
    )?;
    scan_provider_route(
        &fixture,
        utc_reference_request,
        &provider,
        &adapter,
        &recorder,
    )?;

    // Then
    assert_eq!(provider.calls(), 4);
    assert_eq!(provider_route_fingerprint_count(&fixture.store_path)?, 4);
    let dump = provider_route_outcome_dump(&fixture.store_path)?;
    assert_provider_route_dump_hides(
        &dump,
        &[
            "+15555550103",
            "Maybe meet tomorrow?",
            "Maybe meet the project team tomorrow?",
            "{\"kind\"",
            "\"title\"",
            "\"calendar_event\"",
            "Provider supplied title",
        ],
    );
    println!(
        "invalidation_provider_calls={} fingerprint_rows={}",
        provider.calls(),
        provider_route_fingerprint_count(&fixture.store_path)?
    );
    Ok(())
}

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
        dump.contains("2026-06-24T18:53:00[America/Los_Angeles]|America/Los_Angeles"),
        "{dump}"
    );
    Ok(())
}
