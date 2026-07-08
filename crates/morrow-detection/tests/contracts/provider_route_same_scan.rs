use std::error::Error;

use morrow_detection::{DetectionOutcome, DetectionPipeline, ProviderRouteOutcomeKind};

use crate::provider_route_cache_support::{
    trace_reasons, write_intent, StaticProviderRouteCache, UnavailableProvider,
};
use crate::support::{config, message, CollectingRecorder, FakeProvider};

#[test]
fn provider_route_same_scan_coalesces_misses_before_provider_extract() -> Result<(), Box<dyn Error>>
{
    // Given
    let provider = FakeProvider::new(Some(
        "{\"kind\":\"calendar_event\",\"title\":\"Meet Friday afternoon\",\
         \"confidence_millis\":720,\
         \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
         \"anchor_message_guid\":\"msg-same-scan-route\",\
         \"evidence_message_guids\":[\"msg-same-scan-route\"]}",
    ));
    let pipeline = DetectionPipeline::new(&provider);
    let route_write_intent = write_intent("route-same-scan-1", "hash-same-scan-1", "none");
    let cache = StaticProviderRouteCache::miss(Some(route_write_intent.clone()));
    let recorder = CollectingRecorder::default();
    let messages = vec![
        message(
            "chat-1",
            "msg-same-scan-route",
            "Can we meet Friday afternoon?",
            true,
        )?,
        message(
            "chat-1",
            "msg-same-scan-route",
            "Can we meet Friday afternoon?",
            true,
        )?,
    ];
    let config = config(550)?;

    // When
    let report =
        pipeline.detect_with_trace_and_provider_cache(&messages, &config, &recorder, &cache)?;

    // Then
    assert_eq!(provider.calls(), 1);
    assert_eq!(cache.calls(), 2);
    assert_eq!(report.outcomes.len(), 2);
    assert!(matches!(
        report.outcomes.first(),
        Some(DetectionOutcome::Candidate(_))
    ));
    match report.outcomes.get(1) {
        Some(DetectionOutcome::CachedProviderRoute {
            route_fingerprint,
            outcome_kind,
        }) => {
            assert_eq!(route_fingerprint, "route-same-scan-1");
            assert_eq!(*outcome_kind, ProviderRouteOutcomeKind::Candidate);
        }
        Some(DetectionOutcome::Candidate(_)) | Some(DetectionOutcome::QuietLog(_)) | None => {
            return Err("expected second outcome to reuse same-scan provider route".into());
        }
    }
    assert_eq!(
        report.provider_route_write_intents,
        vec![Some(route_write_intent), None]
    );
    let reasons = trace_reasons(&recorder)?;
    assert!(reasons.contains(&"provider_extract_success".to_owned()));
    assert_eq!(
        reasons
            .iter()
            .filter(|reason| reason.as_str() == "provider_route_cache_hit")
            .count(),
        2
    );

    // Given
    let unavailable_provider = UnavailableProvider::default();
    let unavailable_pipeline = DetectionPipeline::new(&unavailable_provider);
    let unavailable_cache = StaticProviderRouteCache::miss(Some(write_intent(
        "route-unavailable-same-scan",
        "hash-unavailable-same-scan",
        "none",
    )));

    // When
    let unavailable_report = unavailable_pipeline.detect_with_trace_and_provider_cache(
        &messages,
        &config,
        &morrow_diagnostics::NoopTraceRecorder,
        &unavailable_cache,
    )?;

    // Then
    assert_eq!(unavailable_provider.calls(), 2);
    assert_eq!(unavailable_cache.calls(), 2);
    assert!(unavailable_report
        .outcomes
        .iter()
        .all(|outcome| matches!(outcome, DetectionOutcome::QuietLog(_))));
    assert_eq!(
        unavailable_report.provider_route_write_intents,
        vec![None, None]
    );
    println!(
        "same_scan_coalesce provider_calls={} cache_calls={} unavailable_provider_calls={} outcomes={:?}",
        provider.calls(),
        cache.calls(),
        unavailable_provider.calls(),
        report.provider_route_write_intents
    );
    Ok(())
}
