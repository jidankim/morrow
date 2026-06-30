use std::error::Error;

use morrow_detection::{
    AiProvider, DetectionPipeline, ProviderError, ProviderRequest, ProviderResponse,
};

use crate::support::{config, message, only_candidate, only_quiet, FakeProvider};

#[derive(Debug, Clone, Copy)]
struct UnavailableProvider;

impl AiProvider for UnavailableProvider {
    fn extract(&self, _request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        Err(ProviderError::Unavailable {
            reason: "network offline".to_owned(),
        })
    }
}

#[test]
fn confidence_thresholds_are_explicit() -> Result<(), Box<dyn Error>> {
    // Given
    let below = FakeProvider::new(Some(
        "{\"kind\":\"calendar_event\",\"title\":\"Low confidence\",\
         \"confidence_millis\":549,\
         \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
         \"anchor_message_guid\":\"msg-ambiguous-1\",\
         \"evidence_message_guids\":[\"msg-ambiguous-1\"]}",
    ));
    let meeting = vec![message(
        "chat-1",
        "msg-ambiguous-1",
        "Can we meet Friday afternoon?",
        true,
    )?];
    let config = config(550)?;

    // When
    let low_report = DetectionPipeline::new(&below).detect(&meeting, &config);

    // Then
    assert_eq!(
        only_quiet(&low_report.outcomes)?.reason,
        "confidence_below_threshold"
    );

    // Given
    let meeting_threshold = FakeProvider::new(Some(
        "{\"kind\":\"calendar_event\",\"title\":\"Threshold confidence\",\
         \"confidence_millis\":550,\
         \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
         \"anchor_message_guid\":\"msg-ambiguous-1\",\
         \"evidence_message_guids\":[\"msg-ambiguous-1\"]}",
    ));

    // When
    let threshold_report = DetectionPipeline::new(&meeting_threshold).detect(&meeting, &config);

    // Then
    assert_eq!(
        only_candidate(&threshold_report.outcomes)?.title,
        "Threshold confidence"
    );
    Ok(())
}

#[test]
fn invalid_provider_confidence_is_rejected() -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(Some(
        "{\"kind\":\"calendar_event\",\"title\":\"Bad confidence\",\
         \"confidence_millis\":1001,\
         \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
         \"anchor_message_guid\":\"msg-ambiguous-1\",\
         \"evidence_message_guids\":[\"msg-ambiguous-1\"]}",
    ));
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-ambiguous-1",
        "Can we meet Friday afternoon?",
        true,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    let quiet = only_quiet(&report.outcomes)?;
    assert_eq!(quiet.reason, "provider_schema_rejected");
    Ok(())
}

#[test]
fn impossible_provider_date_is_rejected() -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(Some(
        "{\"kind\":\"calendar_event\",\"title\":\"Impossible date\",\
         \"confidence_millis\":800,\
         \"normalized_time\":\"2026-09-31T15:00:00[Asia/Seoul]\",\
         \"anchor_message_guid\":\"msg-ambiguous-1\",\
         \"evidence_message_guids\":[\"msg-ambiguous-1\"]}",
    ));
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-ambiguous-1",
        "Can we meet Friday afternoon?",
        true,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    let quiet = only_quiet(&report.outcomes)?;
    assert_eq!(quiet.reason, "provider_schema_rejected");
    Ok(())
}

#[test]
fn provider_normalized_time_rejects_raw_suffix() -> Result<(), Box<dyn Error>> {
    // Given
    let provider =
        FakeProvider::from_owned(Some(provider_payload_at("2026-06-26T15:00:00raw-suffix")));
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-ambiguous-1",
        "Can we meet Friday afternoon?",
        true,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    let quiet = only_quiet(&report.outcomes)?;
    assert_eq!(quiet.reason, "provider_schema_rejected");
    Ok(())
}

#[test]
fn provider_normalized_time_rejects_private_bracketed_zone() -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::from_owned(Some(provider_payload_at(
        "2026-06-26T15:00:00[private_clinic_visit]",
    )));
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-ambiguous-1",
        "Can we meet Friday afternoon?",
        true,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    let quiet = only_quiet(&report.outcomes)?;
    assert_eq!(quiet.reason, "provider_schema_rejected");
    Ok(())
}

#[test]
fn provider_failure_is_quiet_logged_without_successful_candidate() -> Result<(), Box<dyn Error>> {
    // Given
    let provider = UnavailableProvider;
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-provider-down",
        "Can we meet Friday afternoon?",
        true,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    let quiet = only_quiet(&report.outcomes)?;
    assert_eq!(quiet.reason, "provider_unavailable");
    assert_eq!(report.candidates().count(), 0);
    Ok(())
}

fn provider_payload_at(normalized_time: &str) -> String {
    format!(
        "{{\"kind\":\"calendar_event\",\"title\":\"Bad normalized time\",\
         \"confidence_millis\":800,\
         \"normalized_time\":\"{normalized_time}\",\
         \"anchor_message_guid\":\"msg-ambiguous-1\",\
         \"evidence_message_guids\":[\"msg-ambiguous-1\"]}}"
    )
}
