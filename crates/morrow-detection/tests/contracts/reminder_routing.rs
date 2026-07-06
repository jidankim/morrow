use std::{cell::Cell, error::Error};

use morrow_detection::{
    AiProvider, DetectionPipeline, ProviderError, ProviderRequest, ProviderResponse,
};
use morrow_storage::CandidateKind;

use crate::support::{config, message, only_candidate, only_quiet, FakeProvider};

#[path = "reminder_routing/quantity_lists.rs"]
mod quantity_lists;

#[derive(Debug, Clone, Copy)]
struct UnavailableProvider;

impl AiProvider for UnavailableProvider {
    fn extract(&self, _request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        Err(ProviderError::Unavailable {
            reason: "codex provider command timed out".to_owned(),
        })
    }
}

#[derive(Debug)]
struct CountingUnavailableProvider {
    calls: Cell<usize>,
}

impl CountingUnavailableProvider {
    const fn new() -> Self {
        Self {
            calls: Cell::new(0),
        }
    }

    fn calls(&self) -> usize {
        self.calls.get()
    }
}

impl AiProvider for CountingUnavailableProvider {
    fn extract(&self, _request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        self.calls.set(self.calls.get() + 1);
        Err(ProviderError::Unavailable {
            reason: "codex provider command timed out".to_owned(),
        })
    }
}

#[test]
fn scheduling_intent_task_follow_up_by_date_routes_to_provider() -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(Some(
        "{\"kind\":\"task_reminder\",\"title\":\"Follow up with Dana\",\
         \"confidence_millis\":760,\
         \"normalized_time\":\"2026-07-25T09:00:00[Asia/Seoul]\",\
         \"anchor_message_guid\":\"msg-follow-up-date-1\",\
         \"evidence_message_guids\":[\"msg-follow-up-date-1\"]}",
    ));
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-follow-up-date-1",
        "Follow up with Dana by July 25, 2026",
        false,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    assert_eq!(provider.calls(), 1);
    let candidate = only_candidate(&report.outcomes)?;
    assert_eq!(candidate.kind, CandidateKind::TaskReminder);
    assert_eq!(candidate.title, "Follow up with Dana");
    println!(
        "scheduling_intent_task_follow_up_by_date_routes_to_provider provider_calls={} candidate_kind={:?}",
        provider.calls(),
        candidate.kind
    );
    Ok(())
}

#[test]
fn scheduling_intent_task_send_by_date_falls_back_on_provider_unavailable(
) -> Result<(), Box<dyn Error>> {
    // Given
    let provider = UnavailableProvider;
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-send-by-timeout-1",
        "Send the renewal packet by July 25, 2026",
        false,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    let candidate = only_candidate(&report.outcomes)?;
    assert_eq!(candidate.kind, CandidateKind::TaskReminder);
    assert_eq!(candidate.title, "Send the renewal packet by July 25, 2026");
    assert_eq!(candidate.normalized_time, "2026-07-25T23:59:00[Asia/Seoul]");
    assert_eq!(report.quiet_logs().count(), 0);
    println!(
        "scheduling_intent_task_send_by_date_falls_back_on_provider_unavailable candidate_kind={:?} normalized_time={} quiet_logs={}",
        candidate.kind,
        candidate.normalized_time,
        report.quiet_logs().count()
    );
    Ok(())
}

#[test]
fn scheduling_intent_task_by_friday_provider_unavailable_quiets_without_fallback(
) -> Result<(), Box<dyn Error>> {
    // Given
    let provider = CountingUnavailableProvider::new();
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-follow-up-friday-timeout-1",
        "Follow up with Dana by Friday",
        false,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    assert_eq!(provider.calls(), 1);
    let quiet = only_quiet(&report.outcomes)?;
    assert_eq!(quiet.reason, "provider_unavailable");
    assert_eq!(report.candidates().count(), 0);
    println!(
        "scheduling_intent_task_by_friday_provider_unavailable_quiets_without_fallback provider_calls={} quiet_reason={} candidates={}",
        provider.calls(),
        quiet.reason,
        report.candidates().count()
    );
    Ok(())
}

#[test]
fn scheduling_intent_task_existing_finish_review_still_routes_to_provider(
) -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(Some(
        "{\"kind\":\"task_reminder\",\"title\":\"Finish review of the essay\",\
         \"confidence_millis\":760,\
         \"normalized_time\":\"2026-07-25T09:00:00[Asia/Seoul]\",\
         \"anchor_message_guid\":\"msg-finish-review-1\",\
         \"evidence_message_guids\":[\"msg-finish-review-1\"]}",
    ));
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-finish-review-1",
        "Finish review of the essay by July 25, 2026",
        false,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    assert_eq!(provider.calls(), 1);
    let candidate = only_candidate(&report.outcomes)?;
    assert_eq!(candidate.kind, CandidateKind::TaskReminder);
    assert_eq!(candidate.title, "Finish review of the essay");
    assert_eq!(candidate.normalized_time, "2026-07-25T09:00:00[Asia/Seoul]");
    println!(
        "scheduling_intent_task_existing_finish_review_still_routes_to_provider provider_calls={} candidate_kind={:?}",
        provider.calls(),
        candidate.kind
    );
    Ok(())
}

#[test]
fn finish_by_date_wording_creates_reminder_when_provider_unavailable() -> Result<(), Box<dyn Error>>
{
    // Given
    let provider = UnavailableProvider;
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-finish-review-timeout-1",
        "Finish review of the essay by July 25, 2026",
        false,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    let candidate = only_candidate(&report.outcomes)?;
    assert_eq!(candidate.kind, CandidateKind::TaskReminder);
    assert_eq!(
        candidate.title,
        "Finish review of the essay by July 25, 2026"
    );
    assert_eq!(candidate.normalized_time, "2026-07-25T23:59:00[Asia/Seoul]");
    assert_eq!(report.quiet_logs().count(), 0);
    Ok(())
}

#[test]
fn task_due_date_wording_routes_to_provider_as_reminder() -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(Some(
        "{\"kind\":\"task_reminder\",\"title\":\"Essay review\",\
         \"confidence_millis\":740,\
         \"normalized_time\":\"2026-07-25T09:00:00[Asia/Seoul]\",\
         \"anchor_message_guid\":\"msg-essay-review-due-1\",\
         \"evidence_message_guids\":[\"msg-essay-review-due-1\"]}",
    ));
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-essay-review-due-1",
        "Essay review due July 25, 2026",
        false,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    assert_eq!(provider.calls(), 1);
    let candidate = only_candidate(&report.outcomes)?;
    assert_eq!(candidate.kind, CandidateKind::TaskReminder);
    assert_eq!(candidate.title, "Essay review");
    assert_eq!(candidate.normalized_time, "2026-07-25T09:00:00[Asia/Seoul]");
    Ok(())
}

#[test]
fn explicit_task_date_time_creates_reminder_without_provider() -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(None);
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-submit-report-1",
        "Submit report 2026-07-25 09:00",
        false,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    assert_eq!(provider.calls(), 0);
    let candidate = only_candidate(&report.outcomes)?;
    assert_eq!(candidate.kind, CandidateKind::TaskReminder);
    assert_eq!(candidate.title, "Submit report 2026-07-25 09:00");
    assert_eq!(candidate.normalized_time, "2026-07-25T09:00:00[Asia/Seoul]");
    Ok(())
}

#[test]
fn bare_review_with_future_date_stops_without_provider() -> Result<(), Box<dyn Error>> {
    quiet_stop_without_provider("msg-review-date-1", "Review 2026-07-25")
}

#[test]
fn eventful_with_future_date_stops_without_provider() -> Result<(), Box<dyn Error>> {
    quiet_stop_without_provider(
        "msg-eventful-date-1",
        "That eventful week starts 2026-07-25",
    )
}

#[test]
fn callback_with_future_date_stops_without_provider() -> Result<(), Box<dyn Error>> {
    quiet_stop_without_provider(
        "msg-callback-date-1",
        "The callback bug surfaced 2026-07-25",
    )
}

#[test]
fn overdue_with_future_date_stops_without_provider() -> Result<(), Box<dyn Error>> {
    quiet_stop_without_provider(
        "msg-overdue-date-1",
        "The overdue status changed 2026-07-25",
    )
}

fn quiet_stop_without_provider(message_guid: &str, excerpt: &str) -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(None);
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message("chat-1", message_guid, excerpt, false)?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    assert_eq!(provider.calls(), 0);
    let quiet = only_quiet(&report.outcomes)?;
    assert_eq!(quiet.reason, "deterministic_stop:no_scheduling_signal");
    Ok(())
}
