use std::cell::Cell;

use morrow_detection::{AiProvider, ProviderError, ProviderRequest, ProviderResponse};

use super::super::dependencies::RecordingProposalAdapter;
use super::super::message_sqlite::{
    insert_provider_route_companion_message, provider_route_fingerprint_count,
    provider_route_outcome_dump, update_provider_route_companion_message,
};
use super::{
    assert_provider_route_dump_hides, provider_route_request, scan_provider_route,
    ProviderRouteFixture,
};

#[test]
fn provider_route_ledger_invalidation_when_selected_companion_context_changes() -> Result<(), String>
{
    // Given
    let fixture = ProviderRouteFixture::new()?;
    insert_provider_route_companion_message(
        &fixture.messages_db_path,
        "Alpha context: the contract readout is about sponsors.",
    )?;
    let request = provider_route_request()?;
    let provider = ContextSensitiveProvider::default();
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    scan_provider_route(&fixture, request.clone(), &provider, &adapter, &recorder)?;
    update_provider_route_companion_message(
        &fixture.messages_db_path,
        "Beta context: the contract readout is about launch prep.",
    )?;
    scan_provider_route(&fixture, request, &provider, &adapter, &recorder)?;

    // Then
    assert_eq!(
        provider.calls(),
        2,
        "selected companion context changed, so provider route cache must refresh"
    );
    assert_eq!(provider_route_fingerprint_count(&fixture.store_path)?, 2);
    let dump = provider_route_outcome_dump(&fixture.store_path)?;
    assert_provider_route_dump_hides(
        &dump,
        &[
            "Alpha context",
            "Beta context",
            "contract readout is about",
            "{\"kind\"",
            "\"title\"",
            "Alpha-context title",
            "Beta-context title",
        ],
    );
    println!(
        "selected_context_invalidation_provider_calls={} fingerprint_rows={}",
        provider.calls(),
        provider_route_fingerprint_count(&fixture.store_path)?
    );
    Ok(())
}

#[derive(Debug, Default)]
struct ContextSensitiveProvider {
    calls: Cell<usize>,
}

impl ContextSensitiveProvider {
    fn calls(&self) -> usize {
        self.calls.get()
    }
}

impl AiProvider for ContextSensitiveProvider {
    fn extract(&self, request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        self.calls.set(self.calls.get() + 1);
        let title = if request
            .evidence()
            .iter()
            .any(|message| message.excerpt.contains("Alpha context"))
        {
            "Alpha-context title"
        } else {
            "Beta-context title"
        };
        Ok(ProviderResponse::new(&format!(
            "{{\"kind\":\"calendar_event\",\"title\":\"{title}\",\
             \"confidence_millis\":800,\
             \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
             \"anchor_message_guid\":\"beta-provider-route\",\
             \"evidence_message_guids\":[\"alpha-provider-context\",\"beta-provider-route\"]}}"
        )))
    }
}
