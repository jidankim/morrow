use std::error::Error;

use morrow_detection::{DetectionOutcome, DetectionPipeline, ProviderRouteOutcomeKind};

use crate::support::{
    config, message, only_candidate, only_quiet, CollectingRecorder, FakeProvider,
};

use crate::provider_route_cache_support::{
    trace_reasons, write_intent, CacheFailure, StaticProviderRouteCache, UnavailableProvider,
};

#[test]
fn provider_route_cache_hit_skips_provider_extract() -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(None);
    let pipeline = DetectionPipeline::new(&provider);
    let cache = StaticProviderRouteCache::hit("route-hit-1", ProviderRouteOutcomeKind::Candidate);
    let recorder = CollectingRecorder::default();
    let messages = vec![message(
        "chat-1",
        "msg-cache-hit-1",
        "Can we meet Friday afternoon?",
        true,
    )?];
    let config = config(550)?;

    // When
    let report =
        pipeline.detect_with_trace_and_provider_cache(&messages, &config, &recorder, &cache)?;

    // Then
    assert_eq!(provider.calls(), 0);
    assert_eq!(cache.calls(), 1);
    assert_eq!(report.candidates().count(), 0);
    assert_eq!(report.quiet_logs().count(), 0);
    assert_eq!(report.provider_route_write_intents, vec![None]);
    assert_eq!(report.outcomes.len(), 1);
    let outcome = report
        .outcomes
        .first()
        .ok_or("expected cached provider route outcome")?;
    match outcome {
        DetectionOutcome::CachedProviderRoute {
            route_fingerprint,
            outcome_kind,
        } => {
            assert_eq!(route_fingerprint, "route-hit-1");
            assert_eq!(*outcome_kind, ProviderRouteOutcomeKind::Candidate);
        }
        DetectionOutcome::Candidate(_) | DetectionOutcome::QuietLog(_) => {
            return Err("expected cached provider route outcome".into());
        }
    }
    let reasons = trace_reasons(&recorder)?;
    assert!(reasons.contains(&"parser_provider_route_ambiguous_calendar".to_owned()));
    assert!(reasons.contains(&"provider_route_cache_hit".to_owned()));
    assert!(!reasons.contains(&"provider_extract_success".to_owned()));
    assert!(!reasons.contains(&"provider_unavailable".to_owned()));
    println!(
        "provider_route_cache_hit_skips_provider_extract provider_calls={} candidates={} quiet_logs={}",
        provider.calls(),
        report.candidates().count(),
        report.quiet_logs().count()
    );
    Ok(())
}

#[test]
fn provider_route_cache_miss_stages_write_intent_and_calls_provider() -> Result<(), Box<dyn Error>>
{
    // Given
    let provider = FakeProvider::new(Some(
        "{\"kind\":\"calendar_event\",\"title\":\"Meet Friday afternoon\",\
         \"confidence_millis\":720,\
         \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
         \"anchor_message_guid\":\"msg-cache-miss-1\",\
         \"evidence_message_guids\":[\"msg-cache-miss-1\"]}",
    ));
    let pipeline = DetectionPipeline::new(&provider);
    let write_intent = write_intent("route-miss-1", "hash-miss-1", "none");
    let cache = StaticProviderRouteCache::miss(Some(write_intent.clone()));
    let messages = vec![message(
        "chat-1",
        "msg-cache-miss-1",
        "Can we meet Friday afternoon?",
        true,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect_with_trace_and_provider_cache(
        &messages,
        &config,
        &morrow_diagnostics::NoopTraceRecorder,
        &cache,
    )?;

    // Then
    assert_eq!(provider.calls(), 1);
    assert_eq!(cache.calls(), 1);
    let candidate = only_candidate(&report.outcomes)?;
    assert_eq!(candidate.title, "Meet Friday afternoon");
    assert_eq!(
        report.provider_route_write_intents,
        vec![Some(write_intent)]
    );
    let staged = report
        .provider_route_write_intents
        .first()
        .and_then(Option::as_ref)
        .ok_or("expected staged provider-route write intent")?;
    assert_eq!(staged.route_fingerprint, "route-miss-1");
    assert_eq!(
        staged.provider_route_contract_version,
        "provider-route-ledger-v1"
    );
    assert_eq!(
        staged.provider_candidate_schema_version,
        "provider-candidate-schema-v3"
    );
    assert_eq!(staged.evidence_payload_hash, "hash-miss-1");
    assert_eq!(staged.provider_id, "fake-provider");
    assert_eq!(staged.model_id, "offline-contract");
    assert_eq!(staged.prompt_version, "prompt-v1");
    assert_eq!(config.profile.profile_id.as_str(), "list-reminders");
    assert_eq!(config.profile.profile_version.as_str(), "list-reminders-v1");
    assert_eq!(config.profile.routing_mode.as_str(), "explicitOnly");
    assert_eq!(staged.source_excerpt_policy, "include");
    assert_eq!(staged.reference_observed, "2026-06-25T09:00:00[Asia/Seoul]");
    assert_eq!(staged.reference_timezone, "Asia/Seoul");
    assert_eq!(staged.threshold_millis, 550);
    assert_eq!(staged.parser_route, "none");
    Ok(())
}

#[test]
fn provider_unavailable_is_not_cache_hit() -> Result<(), Box<dyn Error>> {
    // Given
    let provider = UnavailableProvider::default();
    let pipeline = DetectionPipeline::new(&provider);
    let cache = StaticProviderRouteCache::miss(Some(write_intent(
        "route-unavailable-1",
        "hash-unavailable-1",
        "none",
    )));
    let messages = vec![message(
        "chat-1",
        "msg-provider-down",
        "Can we meet Friday afternoon?",
        true,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect_with_trace_and_provider_cache(
        &messages,
        &config,
        &morrow_diagnostics::NoopTraceRecorder,
        &cache,
    )?;

    // Then
    assert_eq!(provider.calls(), 1);
    let quiet = only_quiet(&report.outcomes)?;
    assert_eq!(quiet.reason, "provider_unavailable");
    assert_eq!(report.provider_route_write_intents, vec![None]);
    println!(
        "provider_unavailable_is_not_cache_hit provider_calls={} quiet_reason={} write_intents={:?}",
        provider.calls(),
        quiet.reason,
        report.provider_route_write_intents
    );
    Ok(())
}

#[test]
fn provider_route_cache_error_returns_before_provider_extract() -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(None);
    let pipeline = DetectionPipeline::new(&provider);
    let cache = StaticProviderRouteCache::error(CacheFailure);
    let messages = vec![message(
        "chat-1",
        "msg-cache-error-1",
        "Can we meet Friday afternoon?",
        true,
    )?];
    let config = config(550)?;

    // When
    let result = pipeline.detect_with_trace_and_provider_cache(
        &messages,
        &config,
        &morrow_diagnostics::NoopTraceRecorder,
        &cache,
    );

    // Then
    assert!(result.is_err());
    assert_eq!(provider.calls(), 0);
    assert_eq!(cache.calls(), 1);
    Ok(())
}
