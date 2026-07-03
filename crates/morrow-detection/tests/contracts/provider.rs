use std::cell::RefCell;
use std::error::Error;

use morrow_detection::{
    AiProvider, DetectionPipeline, ProviderError, ProviderRequest, ProviderResponse, ReferenceTime,
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

#[derive(Debug, Default)]
struct InspectingProvider {
    request: RefCell<Option<ObservedProviderRequest>>,
}

#[derive(Debug, PartialEq, Eq)]
struct ObservedProviderRequest {
    reference_timezone: String,
    evidence_count: usize,
    anchor_message_guid: String,
    evidence_pointer: String,
    provider_id: String,
}

impl InspectingProvider {
    fn observed_request(&self) -> Result<ObservedProviderRequest, Box<dyn Error>> {
        self.request
            .borrow_mut()
            .take()
            .ok_or_else(|| "provider was not called".into())
    }
}

impl AiProvider for InspectingProvider {
    fn extract(&self, request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        let anchor = &request.evidence()[0];
        self.request.replace(Some(ObservedProviderRequest {
            reference_timezone: request.reference_timezone().to_owned(),
            evidence_count: request.evidence().len(),
            anchor_message_guid: anchor.message_guid.as_str().to_owned(),
            evidence_pointer: anchor.evidence_pointer.clone(),
            provider_id: request.identity().provider_id.clone(),
        }));
        Ok(ProviderResponse::new(
            "{\"kind\":\"calendar_event\",\"title\":\"Observed request\",\
             \"confidence_millis\":800,\
             \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
             \"anchor_message_guid\":\"msg-ambiguous-1\",\
             \"evidence_message_guids\":[\"msg-ambiguous-1\"]}",
        ))
    }
}

#[test]
fn provider_request_exposes_configured_reference_timezone_without_expanding_evidence_surface(
) -> Result<(), Box<dyn Error>> {
    // Given
    let provider = InspectingProvider::default();
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-ambiguous-1",
        "Can we meet Friday afternoon?",
        true,
    )?];
    let mut config = config(550)?;
    config.reference = ReferenceTime::parse("2026-06-25T09:00:00", "America/New_York")?;

    // When
    let _report = pipeline.detect(&messages, &config);

    // Then
    assert_eq!(
        provider.observed_request()?,
        ObservedProviderRequest {
            reference_timezone: "America/New_York".to_owned(),
            evidence_count: 1,
            anchor_message_guid: "msg-ambiguous-1".to_owned(),
            evidence_pointer: "messages://chat-1/msg-ambiguous-1".to_owned(),
            provider_id: "fake-provider".to_owned(),
        }
    );
    Ok(())
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
    assert_eq!(
        only_quiet(&low_report.outcomes)?
            .provider_diagnostic
            .as_deref(),
        None
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
fn provider_normalized_time_rejects_no_zone_output() -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::from_owned(Some(provider_payload_at("2026-06-26T15:00:00")));
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
    assert_eq!(report.candidates().count(), 0);
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
    assert_eq!(
        quiet.provider_diagnostic.as_deref(),
        Some("network offline")
    );
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
