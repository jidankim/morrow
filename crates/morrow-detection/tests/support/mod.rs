use std::cell::Cell;
use std::error::Error;

use morrow_detection::{
    AiProvider, ConfidenceThreshold, DetectionConfig, DetectionOutcome, ProviderError,
    ProviderIdentity, ProviderRequest, ProviderResponse, ReferenceTime, SourceExcerptPolicy,
};
use morrow_messages::{ChatGuid, MessageEvidence, MessageGuid, MessageTimestamp};
use serde::Deserialize;

pub struct FakeProvider {
    response: Option<String>,
    calls: Cell<usize>,
}

impl FakeProvider {
    pub fn new(response: Option<&str>) -> Self {
        Self {
            response: response.map(str::to_owned),
            calls: Cell::new(0),
        }
    }

    pub const fn from_owned(response: Option<String>) -> Self {
        Self {
            response,
            calls: Cell::new(0),
        }
    }

    pub const fn calls(&self) -> usize {
        self.calls.get()
    }
}

impl AiProvider for FakeProvider {
    fn extract(&self, request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        self.calls.set(self.calls.get() + 1);
        assert!(!request.evidence().is_empty());
        let body = self.response.as_deref().unwrap_or(
            "{\"kind\":\"calendar_event\",\"title\":\"Fallback\",\"confidence_millis\":720,\
             \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
             \"anchor_message_guid\":\"msg-ambiguous-1\",\
             \"evidence_message_guids\":[\"msg-ambiguous-1\"]}",
        );
        Ok(ProviderResponse::new(body))
    }
}

#[derive(Deserialize)]
pub struct Fixture {
    pub reference_time: String,
    pub timezone: String,
    pub threshold_millis: i64,
    pub scenarios: Vec<Scenario>,
}

#[derive(Deserialize)]
pub struct Scenario {
    pub name: String,
    pub messages: Vec<MessageFixture>,
    pub provider_response: Option<String>,
    pub expected_candidate_count: usize,
    pub expected_quiet_count: usize,
    pub expected_provider_calls: usize,
}

#[derive(Deserialize)]
pub struct MessageFixture {
    chat_guid: String,
    message_guid: String,
    timestamp: i64,
    participant_count: u16,
    tapback_signal: bool,
    excerpt: String,
    evidence_pointer: String,
}

impl MessageFixture {
    pub fn to_message(&self) -> Result<MessageEvidence, Box<dyn Error>> {
        Ok(MessageEvidence {
            chat_guid: ChatGuid::parse(&self.chat_guid)?,
            message_guid: MessageGuid::parse(&self.message_guid)?,
            timestamp: MessageTimestamp::new(self.timestamp)?,
            participant_count: self.participant_count,
            tapback_signal: self.tapback_signal,
            excerpt: self.excerpt.clone(),
            evidence_pointer: self.evidence_pointer.clone(),
        })
    }
}

pub fn config(threshold_millis: i64) -> Result<DetectionConfig, Box<dyn Error>> {
    Ok(DetectionConfig {
        reference: ReferenceTime::parse("2026-06-25T09:00:00", "Asia/Seoul")?,
        threshold: ConfidenceThreshold::new(threshold_millis)?,
        provider: ProviderIdentity::new("fake-provider", "offline-contract", "prompt-v1")?,
        source_excerpts: SourceExcerptPolicy::Include,
    })
}

pub fn config_from_fixture(fixture: &Fixture) -> Result<DetectionConfig, Box<dyn Error>> {
    Ok(DetectionConfig {
        reference: ReferenceTime::parse(&fixture.reference_time, &fixture.timezone)?,
        threshold: ConfidenceThreshold::new(fixture.threshold_millis)?,
        provider: ProviderIdentity::new("fake-provider", "offline-contract", "prompt-v1")?,
        source_excerpts: SourceExcerptPolicy::Include,
    })
}

pub fn message(
    chat_guid: &str,
    message_guid: &str,
    excerpt: &str,
    tapback_signal: bool,
) -> Result<MessageEvidence, Box<dyn Error>> {
    Ok(MessageEvidence {
        chat_guid: ChatGuid::parse(chat_guid)?,
        message_guid: MessageGuid::parse(message_guid)?,
        timestamp: MessageTimestamp::new(1_782_350_000)?,
        participant_count: 2,
        tapback_signal,
        excerpt: excerpt.to_owned(),
        evidence_pointer: format!("messages://{chat_guid}/{message_guid}"),
    })
}

pub fn only_candidate(
    outcomes: &[DetectionOutcome],
) -> Result<&morrow_storage::CandidateDraft, Box<dyn Error>> {
    assert_eq!(outcomes.len(), 1);
    let outcome = outcomes
        .first()
        .ok_or("expected one detection outcome, got none")?;
    match outcome {
        DetectionOutcome::Candidate(candidate) => Ok(candidate),
        DetectionOutcome::QuietLog(quiet) => {
            Err(format!("expected candidate, got quiet log {}", quiet.reason).into())
        }
    }
}

pub fn only_quiet(
    outcomes: &[DetectionOutcome],
) -> Result<&morrow_storage::QuietLogDraft, Box<dyn Error>> {
    assert_eq!(outcomes.len(), 1);
    let outcome = outcomes
        .first()
        .ok_or("expected one detection outcome, got none")?;
    match outcome {
        DetectionOutcome::QuietLog(quiet) => Ok(quiet),
        DetectionOutcome::Candidate(candidate) => {
            Err(format!("expected quiet log, got candidate {}", candidate.title).into())
        }
    }
}
