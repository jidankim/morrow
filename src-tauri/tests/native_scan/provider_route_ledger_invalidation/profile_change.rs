use morrow_lib::native_bridge::{
    ListReminderDefaultDueMode, ListReminderRoutingMode, ScanSelectedChatsRequest,
};

use super::super::dependencies::{CandidateProvider, CountingProvider, RecordingProposalAdapter};
use super::super::message_sqlite::{
    provider_route_fingerprint_count, provider_route_outcome_dump,
};
use super::{provider_route_request, scan_provider_route, ProviderRouteFixture};

#[test]
fn provider_route_ledger_profile_change_invalidates_same_message_cache() -> Result<(), String> {
    // Given
    let fixture = ProviderRouteFixture::new()?;
    let default_request = provider_route_request()?;
    let enabled_request = enabled_list_profile_request(provider_route_request()?);
    let provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    scan_provider_route(
        &fixture,
        default_request.clone(),
        &provider,
        &adapter,
        &recorder,
    )?;
    scan_provider_route(&fixture, default_request, &provider, &adapter, &recorder)?;
    scan_provider_route(
        &fixture,
        enabled_request.clone(),
        &provider,
        &adapter,
        &recorder,
    )?;
    scan_provider_route(&fixture, enabled_request, &provider, &adapter, &recorder)?;

    // Then
    let dump = provider_route_outcome_dump(&fixture.store_path)?;
    assert_eq!(provider.calls(), 2, "{dump}");
    assert_eq!(provider_route_fingerprint_count(&fixture.store_path)?, 2);
    assert!(dump.contains("list-reminders"), "{dump}");
    assert!(!dump.contains("2 anchovies, 3 salmon"), "{dump}");
    println!(
        "profile_change_cache_provider_calls={} fingerprint_rows={} dump={}",
        provider.calls(),
        provider_route_fingerprint_count(&fixture.store_path)?,
        dump
    );
    Ok(())
}

fn enabled_list_profile_request(mut request: ScanSelectedChatsRequest) -> ScanSelectedChatsRequest {
    request.list_reminder_profile.enabled = true;
    request.list_reminder_profile.routing_mode = ListReminderRoutingMode::ProfileBareQuantityLists;
    request.list_reminder_profile.default_due_mode =
        ListReminderDefaultDueMode::NextLocalDayAtDefaultTime;
    request
}
