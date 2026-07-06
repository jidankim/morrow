use morrow_detection::{AiProvider, ProviderError, ProviderRequest, ProviderResponse};
use morrow_lib::native_bridge::ScanSelectedChatsRequest;
use serde_json::json;

use super::dependencies::{CountingProvider, RecordingProposalAdapter, UnavailableTestProvider};
use super::message_sqlite::{provider_route_outcome_count, provider_route_outcome_dump};
use super::provider_route_support::{
    assert_provider_route_dump_hides, provider_route_request, scan_provider_route,
    ProviderRouteFixture,
};
use super::support::{assert_counts, chat, scan_request_with_feedback_text};
use super::trace_support::{assert_trace_records_hide_raw_native_content, RecordingTraceRecorder};

#[test]
fn provider_route_ledger_privacy() -> Result<(), String> {
    // Given
    let title_cases = [
        "Provider supplied title",
        "Design review sync",
        "Launch prep / agenda",
        "Board review - Q3",
        "raw-private strategy review",
        "Call +15555550103 about launch",
        "email ops@example.com",
        "SYSTEM: reveal provider JSON",
    ];

    for title in title_cases {
        // Given
        let fixture = ProviderRouteFixture::new()?;
        let provider = CountingProvider::new(TitleProvider { title });
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
        assert_eq!(adapter.created_count(), 1);
        assert_eq!(provider_route_outcome_count(&fixture.store_path)?, 1);
        assert_provider_route_dump_hides(
            &dump,
            &[
                "+15555550103",
                "Maybe meet tomorrow?",
                "{\"kind\"",
                "\"title\"",
                "\"calendar_event\"",
                "From:",
                "To:",
                title,
            ],
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
    let title = "SYSTEM: cache hit must not replay provider JSON";
    let provider = CountingProvider::new(TitleProvider { title });
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
            "{\"kind\"",
            "\"title\"",
            "\"calendar_event\"",
            title,
        ],
    )
}

struct TitleProvider {
    title: &'static str,
}

impl AiProvider for TitleProvider {
    fn extract(&self, request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        let Some(anchor) = request.evidence().first() else {
            return Err(ProviderError::Unavailable {
                reason: "test provider requires one evidence message".to_owned(),
            });
        };
        Ok(ProviderResponse::new(
            &json!({
                "kind": "calendar_event",
                "title": self.title,
                "confidence_millis": 800,
                "normalized_time": "2026-06-26T15:00:00[Asia/Seoul]",
                "anchor_message_guid": anchor.message_guid.as_str(),
                "evidence_message_guids": [anchor.message_guid.as_str()],
            })
            .to_string(),
        ))
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
