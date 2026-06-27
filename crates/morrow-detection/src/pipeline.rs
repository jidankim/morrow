use morrow_messages::MessageEvidence;
use morrow_storage::{CandidateDraft, QuietLogDraft};

use crate::parser::{classify, GateDecision, ParsedCandidate};
use crate::provider::{AiProvider, ProviderRequest};
use crate::schema::{parse_provider_candidate, ProviderCandidate, SchemaRejection};
use crate::types::{DetectionConfig, SourceExcerptPolicy};

const HIDDEN_SOURCE_EXCERPT: &str = "Source excerpt hidden by settings.";

#[derive(Debug, Clone)]
pub enum DetectionOutcome {
    Candidate(CandidateDraft),
    QuietLog(QuietLogDraft),
}

#[derive(Debug, Clone, Default)]
pub struct DetectionReport {
    pub outcomes: Vec<DetectionOutcome>,
}

impl DetectionReport {
    pub fn candidates(&self) -> impl Iterator<Item = &CandidateDraft> {
        self.outcomes.iter().filter_map(|outcome| match outcome {
            DetectionOutcome::Candidate(candidate) => Some(candidate),
            DetectionOutcome::QuietLog(_) => None,
        })
    }

    pub fn quiet_logs(&self) -> impl Iterator<Item = &QuietLogDraft> {
        self.outcomes.iter().filter_map(|outcome| match outcome {
            DetectionOutcome::QuietLog(quiet) => Some(quiet),
            DetectionOutcome::Candidate(_) => None,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DetectionPipeline<'a, P> {
    provider: &'a P,
}

impl<'a, P: AiProvider> DetectionPipeline<'a, P> {
    pub const fn new(provider: &'a P) -> Self {
        Self { provider }
    }

    pub fn detect(
        &self,
        messages: &[MessageEvidence],
        config: &DetectionConfig,
    ) -> DetectionReport {
        let outcomes = messages
            .iter()
            .map(|message| self.detect_one(message, config))
            .collect();
        DetectionReport { outcomes }
    }

    fn detect_one(&self, message: &MessageEvidence, config: &DetectionConfig) -> DetectionOutcome {
        match classify(message, config) {
            GateDecision::Stop { reason } => quiet(message, reason, config.source_excerpts),
            GateDecision::Candidate(parsed) => {
                candidate_from_parsed(message, parsed, &message.excerpt, config)
            }
            GateDecision::ProviderRoute { parser_time } => {
                self.detect_with_provider(message, parser_time, config)
            }
        }
    }

    fn detect_with_provider(
        &self,
        message: &MessageEvidence,
        parser_time: Option<crate::types::CivilDateTime>,
        config: &DetectionConfig,
    ) -> DetectionOutcome {
        let evidence = std::slice::from_ref(message);
        let response = match self
            .provider
            .extract(ProviderRequest::new(evidence, &config.provider))
        {
            Ok(response) => response,
            Err(_) => return quiet(message, "provider_unavailable", config.source_excerpts),
        };
        match parse_provider_candidate(response.raw_json(), evidence, parser_time, config) {
            Ok(provider_candidate)
                if provider_candidate.parsed.confidence_millis >= config.threshold.as_i64() =>
            {
                candidate_from_provider(message, provider_candidate, config.source_excerpts)
            }
            Ok(_) => quiet(
                message,
                "confidence_below_threshold",
                config.source_excerpts,
            ),
            Err(rejection) => quiet(message, rejection.reason(), config.source_excerpts),
        }
    }
}

impl SchemaRejection {
    const fn reason(self) -> &'static str {
        match self {
            Self::InvalidJson => "provider_invalid_json",
            Self::InvalidSchema => "provider_schema_rejected",
            Self::HallucinatedEvidence => "provider_hallucinated_evidence",
            Self::ParserConflict => "parser_provider_time_conflict",
        }
    }
}

fn candidate_from_provider(
    anchor: &MessageEvidence,
    provider_candidate: ProviderCandidate,
    source_excerpts: SourceExcerptPolicy,
) -> DetectionOutcome {
    DetectionOutcome::Candidate(CandidateDraft {
        kind: provider_candidate.parsed.kind,
        chat_guid: anchor.chat_guid.as_str().to_owned(),
        anchor_message_guid: provider_candidate.anchor_message_guid,
        title: provider_candidate.title,
        confidence_millis: provider_candidate.parsed.confidence_millis,
        normalized_time: provider_candidate.normalized_time,
        evidence_excerpt: excerpt_for_policy(&anchor.excerpt, source_excerpts),
        observed_at: anchor.timestamp.as_i64(),
    })
}

fn candidate_from_parsed(
    anchor: &MessageEvidence,
    parsed: ParsedCandidate,
    title_source: &str,
    config: &DetectionConfig,
) -> DetectionOutcome {
    DetectionOutcome::Candidate(CandidateDraft {
        kind: parsed.kind,
        chat_guid: anchor.chat_guid.as_str().to_owned(),
        anchor_message_guid: anchor.message_guid.as_str().to_owned(),
        title: title_from_excerpt(title_source),
        confidence_millis: parsed.confidence_millis,
        normalized_time: parsed.time.normalized(&config.reference.timezone),
        evidence_excerpt: excerpt_for_policy(&anchor.excerpt, config.source_excerpts),
        observed_at: anchor.timestamp.as_i64(),
    })
}

fn quiet(
    anchor: &MessageEvidence,
    reason: &'static str,
    source_excerpts: SourceExcerptPolicy,
) -> DetectionOutcome {
    DetectionOutcome::QuietLog(QuietLogDraft {
        chat_guid: anchor.chat_guid.as_str().to_owned(),
        anchor_message_guid: anchor.message_guid.as_str().to_owned(),
        reason: reason.to_owned(),
        excerpt: excerpt_for_policy(&anchor.excerpt, source_excerpts),
        created_at: anchor.timestamp.as_i64(),
    })
}

fn excerpt_for_policy(excerpt: &str, policy: SourceExcerptPolicy) -> String {
    match policy {
        SourceExcerptPolicy::Include => short_excerpt(excerpt),
        SourceExcerptPolicy::Hide => HIDDEN_SOURCE_EXCERPT.to_owned(),
    }
}

fn title_from_excerpt(excerpt: &str) -> String {
    bounded_text(excerpt, 120)
}

fn short_excerpt(excerpt: &str) -> String {
    let three_lines = excerpt.lines().take(3).collect::<Vec<_>>().join(" ");
    bounded_text(&three_lines, 280)
}

fn bounded_text(value: &str, max_bytes: usize) -> String {
    let mut output = String::new();
    for ch in value.chars() {
        let next_len = output.len() + ch.len_utf8();
        if next_len > max_bytes {
            break;
        }
        output.push(ch);
    }
    output
}
