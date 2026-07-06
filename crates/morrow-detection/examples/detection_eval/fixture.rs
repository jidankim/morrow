use std::cell::{Cell, RefCell};
use std::error::Error;
use std::path::Path;

use morrow_detection::{
    AiProvider, ConfidenceThreshold, DetectionConfig, ProviderError, ProviderIdentity,
    ProviderRequest, ProviderResponse, ReferenceTime, SourceExcerptPolicy,
};
use morrow_diagnostics::{
    validate_trace_record_privacy, TraceComponent, TraceDecision, TraceOperation, TraceOutcome,
    TracePrivacyTier, TraceRecord, TraceRecorder, TraceRecorderError,
};
use morrow_messages::{ChatGuid, MessageEvidence, MessageGuid, MessageTimestamp};
use serde::Deserialize;

const DEFAULT_PROVIDER_RESPONSE: &str = "{\"kind\":\"calendar_event\",\"title\":\"Fallback\",\
\"confidence_millis\":720,\"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
\"anchor_message_guid\":\"msg-ambiguous-1\",\"evidence_message_guids\":[\"msg-ambiguous-1\"]}";

#[derive(Clone, Deserialize)]
pub struct Fixture {
    pub reference_time: String,
    pub timezone: String,
    pub threshold_millis: i64,
    pub scenarios: Vec<Scenario>,
}

#[derive(Clone, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub messages: Vec<MessageFixture>,
    pub provider_response: Option<String>,
    pub expected_candidate_count: usize,
    pub expected_quiet_count: usize,
    pub expected_provider_calls: usize,
    pub expected_trace: Vec<ExpectedTraceStep>,
}

impl Scenario {
    pub fn to_messages(&self) -> Result<Vec<MessageEvidence>, Box<dyn Error>> {
        self.messages
            .iter()
            .map(MessageFixture::to_message)
            .collect::<Result<Vec<_>, _>>()
    }

    pub fn is_unsafe_action(&self) -> bool {
        self.name.contains("unsafe") || self.name.contains("side_effect")
    }

    pub fn is_prompt_injection(&self) -> bool {
        self.name.contains("prompt_injection")
    }
}

#[derive(Clone, Deserialize)]
pub struct ExpectedTraceStep {
    pub component: TraceComponent,
    pub operation: TraceOperation,
    pub decision: Option<TraceDecision>,
    pub outcome: TraceOutcome,
    pub reason_code: Option<String>,
    pub privacy_tier: TracePrivacyTier,
}

#[derive(Clone, Deserialize)]
pub struct MessageFixture {
    pub chat_guid: String,
    pub message_guid: String,
    pub timestamp: i64,
    pub participant_count: u16,
    pub tapback_signal: bool,
    pub excerpt: String,
    pub evidence_pointer: String,
}

pub fn config_from_fixture(fixture: &Fixture) -> Result<DetectionConfig, Box<dyn Error>> {
    Ok(DetectionConfig {
        reference: ReferenceTime::parse(&fixture.reference_time, &fixture.timezone)?,
        threshold: ConfidenceThreshold::new(fixture.threshold_millis)?,
        provider: ProviderIdentity::new("fake-provider", "offline-contract", "prompt-v1")?,
        profile: morrow_detection::ListReminderProfile::disabled(),
        source_excerpts: SourceExcerptPolicy::Hide,
    })
}

pub fn dataset_family(path: &Path) -> String {
    if path
        .components()
        .any(|part| part.as_os_str() == "adversarial")
    {
        "morrow-adversarial".to_owned()
    } else {
        "morrow-golden".to_owned()
    }
}

pub fn dataset_version(path: &Path) -> Result<String, Box<dyn Error>> {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or("fixture path must have a UTF-8 file stem")?;
    Ok(format!("{}:{stem}:v1", dataset_family(path)))
}

pub struct FixtureProvider {
    response: Option<String>,
    calls: Cell<usize>,
}

impl FixtureProvider {
    pub const fn new(response: Option<String>) -> Self {
        Self {
            response,
            calls: Cell::new(0),
        }
    }

    pub const fn calls(&self) -> usize {
        self.calls.get()
    }
}

impl AiProvider for FixtureProvider {
    fn extract(&self, request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        self.calls.set(self.calls.get() + 1);
        if request.evidence().is_empty() {
            return Err(ProviderError::Unavailable {
                reason: "empty evidence".to_owned(),
            });
        }
        Ok(ProviderResponse::new(
            self.response
                .as_deref()
                .unwrap_or(DEFAULT_PROVIDER_RESPONSE),
        ))
    }
}

#[derive(Default)]
pub struct CollectingRecorder {
    records: RefCell<Vec<TraceRecord>>,
}

impl CollectingRecorder {
    pub fn records(&self) -> Result<Vec<TraceRecord>, Box<dyn Error>> {
        let records = self.records.borrow().clone();
        for record in &records {
            validate_trace_record_privacy(record)?;
        }
        Ok(records)
    }
}

impl TraceRecorder for CollectingRecorder {
    fn record(&self, record: &TraceRecord) -> Result<(), TraceRecorderError> {
        if validate_trace_record_privacy(record).is_err() {
            return Err(TraceRecorderError::Unavailable);
        }
        self.records.borrow_mut().push(record.clone());
        Ok(())
    }
}

impl MessageFixture {
    fn to_message(&self) -> Result<MessageEvidence, Box<dyn Error>> {
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
