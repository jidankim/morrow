use std::error::Error;

use morrow_detection::{
    AiProvider, DetectionPipeline, ProviderError, ProviderRequest, ProviderResponse,
};
use morrow_storage::CandidateKind;

use crate::support::{config, message, only_candidate, only_quiet, FakeProvider};

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
    let provider = FakeProvider::new(None);
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
    assert_eq!(provider.calls(), 0);
    let candidate = only_candidate(&report.outcomes)?;
    assert_eq!(candidate.kind, CandidateKind::CalendarEvent);
    assert_eq!(candidate.anchor_message_guid, "msg-coffee-sync-1");
    assert_eq!(candidate.normalized_time, "2026-07-17T09:30:00[Asia/Seoul]");
    assert_eq!(candidate.confidence_millis, 850);
    println!(
        "coffee_sync_deterministic provider_calls={} normalized_time={}",
        provider.calls(),
        candidate.normalized_time
    );
    Ok(())
}

#[test]
fn scheduling_intent_parser_keeps_coffee_sync_explicit_time_deterministic(
) -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(None);
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
    assert_eq!(provider.calls(), 0);
    let candidate = only_candidate(&report.outcomes)?;
    assert_eq!(candidate.kind, CandidateKind::CalendarEvent);
    assert_eq!(candidate.anchor_message_guid, "msg-coffee-sync-parser-1");
    assert_eq!(candidate.normalized_time, "2026-07-17T09:30:00[Asia/Seoul]");
    println!(
        "scheduling_intent_parser_keeps_coffee_sync_explicit_time_deterministic provider_calls={} normalized_time={}",
        provider.calls(),
        candidate.normalized_time
    );
    Ok(())
}

#[test]
fn scheduling_intent_parser_routes_catch_up_friday_afternoon_to_provider(
) -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(Some(
        "{\"kind\":\"calendar_event\",\"title\":\"Catch up Friday afternoon\",\
         \"confidence_millis\":720,\
         \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
         \"anchor_message_guid\":\"msg-catch-up-parser-1\",\
         \"evidence_message_guids\":[\"msg-catch-up-parser-1\"]}",
    ));
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-catch-up-parser-1",
        "Catch up Friday afternoon?",
        false,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    assert_eq!(provider.calls(), 1);
    let candidate = only_candidate(&report.outcomes)?;
    assert_eq!(candidate.kind, CandidateKind::CalendarEvent);
    assert_eq!(candidate.title, "Catch up Friday afternoon");
    println!(
        "scheduling_intent_parser_routes_catch_up_friday_afternoon_to_provider provider_calls={}",
        provider.calls()
    );
    Ok(())
}

#[test]
fn scheduling_intent_parser_routes_touch_base_friday_afternoon_to_provider(
) -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(Some(
        "{\"kind\":\"calendar_event\",\"title\":\"Touch base Friday afternoon\",\
         \"confidence_millis\":720,\
         \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
         \"anchor_message_guid\":\"msg-touch-base-parser-1\",\
         \"evidence_message_guids\":[\"msg-touch-base-parser-1\"]}",
    ));
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-touch-base-parser-1",
        "Touch base Friday afternoon?",
        false,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    assert_eq!(provider.calls(), 1);
    let candidate = only_candidate(&report.outcomes)?;
    assert_eq!(candidate.kind, CandidateKind::CalendarEvent);
    assert_eq!(candidate.title, "Touch base Friday afternoon");
    println!(
        "scheduling_intent_parser_routes_touch_base_friday_afternoon_to_provider provider_calls={}",
        provider.calls()
    );
    Ok(())
}

#[test]
fn scheduling_intent_parser_stops_great_sync_yesterday_without_provider(
) -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(None);
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-great-sync-parser-1",
        "Great sync yesterday",
        false,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    assert_eq!(provider.calls(), 0);
    let quiet = only_quiet(&report.outcomes)?;
    assert!(quiet.reason.starts_with("deterministic_stop:"));
    println!(
        "scheduling_intent_parser_stops_great_sync_yesterday_without_provider provider_calls={} reason={}",
        provider.calls(),
        quiet.reason
    );
    Ok(())
}

#[test]
fn scheduling_intent_parser_stops_modal_may_call_without_provider() -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(None);
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-modal-may-parser-1",
        "May I call you?",
        false,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    assert_eq!(provider.calls(), 0);
    let quiet = only_quiet(&report.outcomes)?;
    assert_eq!(quiet.reason, "deterministic_stop:no_scheduling_signal");
    println!(
        "scheduling_intent_parser_stops_modal_may_call_without_provider provider_calls={} reason={}",
        provider.calls(),
        quiet.reason
    );
    Ok(())
}

#[test]
fn scheduling_intent_parser_preserves_may_deadline_fallback() -> Result<(), Box<dyn Error>> {
    // Given
    let provider = UnavailableProvider;
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-follow-up-may-parser-1",
        "Follow up by May 12, 2027",
        false,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    let candidate = only_candidate(&report.outcomes)?;
    assert_eq!(candidate.kind, CandidateKind::TaskReminder);
    assert_eq!(candidate.title, "Follow up by May 12, 2027");
    assert_eq!(candidate.normalized_time, "2027-05-12T23:59:00[Asia/Seoul]");
    assert_eq!(report.quiet_logs().count(), 0);
    println!(
        "scheduling_intent_parser_preserves_may_deadline_fallback normalized_time={} quiet_logs={}",
        candidate.normalized_time,
        report.quiet_logs().count()
    );
    Ok(())
}

#[test]
fn scheduling_intent_parser_stops_bare_review_future_date_without_provider(
) -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(None);
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-bare-review-parser-1",
        "Review 2026-07-25",
        false,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    assert_eq!(provider.calls(), 0);
    let quiet = only_quiet(&report.outcomes)?;
    assert_eq!(quiet.reason, "deterministic_stop:no_scheduling_signal");
    println!(
        "scheduling_intent_parser_stops_bare_review_future_date_without_provider provider_calls={} reason={}",
        provider.calls(),
        quiet.reason
    );
    Ok(())
}

#[test]
fn scheduling_intent_parser_stops_digit_only_task_words_without_provider(
) -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(None);
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-build-id-parser-1",
        "Complete build 12345",
        false,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    assert_eq!(provider.calls(), 0);
    let quiet = only_quiet(&report.outcomes)?;
    assert_eq!(quiet.reason, "deterministic_stop:no_scheduling_signal");
    println!(
        "scheduling_intent_parser_stops_digit_only_task_words_without_provider provider_calls={} reason={}",
        provider.calls(),
        quiet.reason
    );
    Ok(())
}
