use super::super::dependencies::{CandidateProvider, CountingProvider, RecordingProposalAdapter};
use super::super::message_sqlite::{
    corrupt_provider_candidate_schema_version, corrupt_provider_route_contract_version,
    provider_route_outcome_count, provider_route_outcome_dump, set_provider_route_prompt_version,
};
use super::{provider_route_request, scan_provider_route, ProviderRouteFixture};

#[test]
fn scheduling_intent_prompt_version_scan_v2_row_invalidates_under_scan_v3() -> Result<(), String> {
    // Given
    let fixture = ProviderRouteFixture::new()?;
    let request = provider_route_request()?;
    let provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;
    scan_provider_route(&fixture, request.clone(), &provider, &adapter, &recorder)?;
    set_provider_route_prompt_version(&fixture.store_path, "scan-v2")?;

    // When
    scan_provider_route(&fixture, request, &provider, &adapter, &recorder)?;

    // Then
    assert_eq!(provider.calls(), 2);
    assert_eq!(provider_route_outcome_count(&fixture.store_path)?, 1);
    let dump = provider_route_outcome_dump(&fixture.store_path)?;
    assert!(dump.contains("|scan-v3|"), "{dump}");
    assert!(!dump.contains("|scan-v2|"), "{dump}");
    println!(
        "prompt_version_invalidation provider_calls={} rows={} dump={}",
        provider.calls(),
        provider_route_outcome_count(&fixture.store_path)?,
        dump
    );
    Ok(())
}

#[test]
fn provider_route_ledger_contract_and_schema_mismatch_invalidates() -> Result<(), String> {
    // Given
    let contract_fixture = ProviderRouteFixture::new()?;
    let request = provider_route_request()?;
    let contract_provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;
    scan_provider_route(
        &contract_fixture,
        request.clone(),
        &contract_provider,
        &adapter,
        &recorder,
    )?;
    corrupt_provider_route_contract_version(&contract_fixture.store_path)?;

    // When
    scan_provider_route(
        &contract_fixture,
        request.clone(),
        &contract_provider,
        &adapter,
        &recorder,
    )?;

    // Then
    assert_eq!(contract_provider.calls(), 2);
    assert!(
        provider_route_outcome_dump(&contract_fixture.store_path)?
            .contains("provider-route-ledger-v1"),
        "contract version was not refreshed"
    );

    // Given
    let schema_fixture = ProviderRouteFixture::new()?;
    let schema_provider = CountingProvider::new(CandidateProvider);
    scan_provider_route(
        &schema_fixture,
        request.clone(),
        &schema_provider,
        &adapter,
        &recorder,
    )?;
    corrupt_provider_candidate_schema_version(&schema_fixture.store_path)?;

    // When
    scan_provider_route(
        &schema_fixture,
        request,
        &schema_provider,
        &adapter,
        &recorder,
    )?;

    // Then
    assert_eq!(schema_provider.calls(), 2);
    assert!(
        provider_route_outcome_dump(&schema_fixture.store_path)?
            .contains("provider-candidate-schema-v3"),
        "candidate schema version was not refreshed"
    );
    Ok(())
}
