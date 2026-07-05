use std::error::Error;

use morrow_detection::DetectionPipeline;
use morrow_storage::CandidateKind;

use crate::support::{config, message, only_candidate, only_quiet, FakeProvider};

#[test]
fn finish_by_date_wording_routes_to_provider_as_reminder() -> Result<(), Box<dyn Error>> {
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
