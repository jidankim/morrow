use std::error::Error;

use morrow_detection::{
    AiProvider, ConfidenceThreshold, DetectionConfig, DetectionPipeline, ProviderError,
    ProviderIdentity, ProviderRequest, ProviderResponse, ReferenceTime, SourceExcerptPolicy,
};
use morrow_messages::{ChatGuid, MessageEvidence, MessageGuid, MessageTimestamp};

use crate::harness_error::HarnessError;

#[derive(Debug, Clone)]
pub(crate) struct ProviderSummary {
    pub(crate) quiet_reason: String,
    pub(crate) candidates: usize,
}

#[derive(Debug, Clone, Copy)]
struct UnavailableProvider;

impl AiProvider for UnavailableProvider {
    fn extract(&self, _request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        Err(ProviderError::Unavailable {
            reason: "offline harness".to_owned(),
        })
    }
}

pub(crate) fn run() -> Result<ProviderSummary, Box<dyn Error>> {
    let provider = UnavailableProvider;
    let report = DetectionPipeline::new(&provider).detect(&[message()?], &config()?);
    let quiet_reason = report
        .quiet_logs()
        .next()
        .map(|quiet| quiet.reason.clone())
        .ok_or_else(|| std::io::Error::other("missing quiet log"))?;
    if quiet_reason != "provider_unavailable" {
        return Err(HarnessError::UnexpectedOutcome {
            scenario: "provider_failures",
            expected: "provider_unavailable",
            actual: quiet_reason,
        }
        .into());
    }
    let candidates = report.candidates().count();
    if candidates != 0 {
        return Err(HarnessError::CountMismatch {
            scenario: "provider_failures",
            field: "candidates",
            expected: 0,
            actual: candidates,
        }
        .into());
    }
    Ok(ProviderSummary {
        quiet_reason,
        candidates,
    })
}

fn config() -> Result<DetectionConfig, Box<dyn Error>> {
    Ok(DetectionConfig {
        reference: ReferenceTime::parse("2026-06-25T09:00:00", "Asia/Seoul")?,
        threshold: ConfidenceThreshold::new(550)?,
        provider: ProviderIdentity::new("fake-provider", "offline-contract", "prompt-v1")?,
        source_excerpts: SourceExcerptPolicy::Include,
    })
}

fn message() -> Result<MessageEvidence, Box<dyn Error>> {
    Ok(MessageEvidence {
        chat_guid: ChatGuid::parse("chat-provider")?,
        message_guid: MessageGuid::parse("msg-provider")?,
        timestamp: MessageTimestamp::new(1_782_350_000)?,
        participant_count: 2,
        tapback_signal: true,
        excerpt: "Can we meet Friday afternoon?".to_owned(),
        evidence_pointer: "messages://chat-provider/msg-provider".to_owned(),
    })
}
