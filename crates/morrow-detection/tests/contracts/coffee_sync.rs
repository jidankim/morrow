use std::error::Error;

use morrow_detection::{
    AiProvider, DetectionPipeline, ProviderError, ProviderRequest, ProviderResponse,
};
use morrow_storage::CandidateKind;

use crate::support::{config, message, only_candidate, FakeProvider};

#[derive(Debug, Clone, Copy)]
struct UnavailableProvider;

impl AiProvider for UnavailableProvider {
    fn extract(&self, _request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        Err(ProviderError::Unavailable {
            reason: "codex provider command timed out".to_owned(),
        })
    }
}

#[test]
fn coffee_sync_with_explicit_meridiem_time_creates_calendar_candidate() -> Result<(), Box<dyn Error>>
{
    // Given
    let provider = FakeProvider::new(Some(
        "{\"kind\":\"calendar_event\",\"title\":\"coffee sync\",\
         \"confidence_millis\":860,\
         \"normalized_time\":\"2026-07-17T09:30:00[Asia/Seoul]\",\
         \"anchor_message_guid\":\"msg-coffee-sync-1\",\
         \"evidence_message_guids\":[\"msg-coffee-sync-1\"]}",
    ));
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-coffee-sync-1",
        "Morrow QA live receipt test: coffee sync on 2026-07-17 at 9:30 AM for 30 minutes",
        false,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    assert_eq!(provider.calls(), 1);
    let candidate = only_candidate(&report.outcomes)?;
    assert_eq!(candidate.kind, CandidateKind::CalendarEvent);
    assert_eq!(candidate.anchor_message_guid, "msg-coffee-sync-1");
    assert_eq!(candidate.title, "coffee sync");
    assert_eq!(candidate.normalized_time, "2026-07-17T09:30:00[Asia/Seoul]");
    assert_eq!(candidate.confidence_millis, 860);
    println!(
        "coffee_sync_provider_route provider_calls={} normalized_time={}",
        provider.calls(),
        candidate.normalized_time
    );
    Ok(())
}

#[test]
fn scheduling_intent_parser_routes_coffee_sync_explicit_time_to_provider(
) -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(Some(
        "{\"kind\":\"calendar_event\",\"title\":\"Coffee sync\",\
         \"confidence_millis\":860,\
         \"normalized_time\":\"2026-07-17T09:30:00[Asia/Seoul]\",\
         \"anchor_message_guid\":\"msg-coffee-sync-parser-1\",\
         \"evidence_message_guids\":[\"msg-coffee-sync-parser-1\"]}",
    ));
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-coffee-sync-parser-1",
        "Coffee sync on 2026-07-17 at 9:30 AM",
        false,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    assert_eq!(provider.calls(), 1);
    let candidate = only_candidate(&report.outcomes)?;
    assert_eq!(candidate.kind, CandidateKind::CalendarEvent);
    assert_eq!(candidate.anchor_message_guid, "msg-coffee-sync-parser-1");
    assert_eq!(candidate.title, "Coffee sync");
    assert_eq!(candidate.normalized_time, "2026-07-17T09:30:00[Asia/Seoul]");
    println!(
        "scheduling_intent_parser_routes_coffee_sync_explicit_time_to_provider provider_calls={} normalized_time={}",
        provider.calls(),
        candidate.normalized_time
    );
    Ok(())
}

#[test]
fn scheduling_intent_parser_uses_subject_title_when_coffee_sync_provider_unavailable(
) -> Result<(), Box<dyn Error>> {
    // Given
    let provider = UnavailableProvider;
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-coffee-sync-unavailable-parser-1",
        "Morrow QA live receipt test: coffee sync on 2026-07-17 at 9:30 AM for 30 minutes",
        false,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    let candidate = only_candidate(&report.outcomes)?;
    assert_eq!(candidate.kind, CandidateKind::CalendarEvent);
    assert_eq!(
        candidate.anchor_message_guid,
        "msg-coffee-sync-unavailable-parser-1"
    );
    assert_eq!(candidate.title, "coffee sync");
    assert_eq!(candidate.normalized_time, "2026-07-17T09:30:00[Asia/Seoul]");
    assert_eq!(report.quiet_logs().count(), 0);
    println!(
        "scheduling_intent_parser_uses_subject_title_when_coffee_sync_provider_unavailable normalized_time={} quiet_logs={}",
        candidate.normalized_time,
        report.quiet_logs().count()
    );
    Ok(())
}
