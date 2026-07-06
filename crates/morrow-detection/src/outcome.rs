use morrow_messages::MessageEvidence;
use morrow_storage::{CandidateDraft, QuietLogDraft};

use crate::parser::ParsedCandidate;
use crate::provider_route_cache::{ProviderRouteOutcomeKind, ProviderRouteWriteIntent};
use crate::schema::ProviderCandidate;
use crate::types::{DetectionConfig, SourceExcerptPolicy};

const HIDDEN_SOURCE_EXCERPT: &str = "Source excerpt hidden by settings.";

#[derive(Debug, Clone)]
pub enum DetectionOutcome {
    Candidate(CandidateDraft),
    QuietLog(QuietLogDraft),
    CachedProviderRoute {
        route_fingerprint: String,
        outcome_kind: ProviderRouteOutcomeKind,
    },
}

#[derive(Debug, Clone, Default)]
pub struct DetectionReport {
    pub outcomes: Vec<DetectionOutcome>,
    pub provider_route_write_intents: Vec<Option<ProviderRouteWriteIntent>>,
}

impl DetectionReport {
    pub fn candidates(&self) -> impl Iterator<Item = &CandidateDraft> {
        self.outcomes.iter().filter_map(|outcome| match outcome {
            DetectionOutcome::Candidate(candidate) => Some(candidate),
            DetectionOutcome::QuietLog(_) | DetectionOutcome::CachedProviderRoute { .. } => None,
        })
    }

    pub fn quiet_logs(&self) -> impl Iterator<Item = &QuietLogDraft> {
        self.outcomes.iter().filter_map(|outcome| match outcome {
            DetectionOutcome::QuietLog(quiet) => Some(quiet),
            DetectionOutcome::Candidate(_) | DetectionOutcome::CachedProviderRoute { .. } => None,
        })
    }
}

pub(crate) fn candidate_from_provider(
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

pub(crate) fn candidate_from_parsed(
    anchor: &MessageEvidence,
    parsed: ParsedCandidate,
    title_source: &str,
    config: &DetectionConfig,
) -> DetectionOutcome {
    let title = parsed
        .title_source
        .as_deref()
        .map_or_else(|| title_from_excerpt(title_source), title_from_excerpt);
    DetectionOutcome::Candidate(CandidateDraft {
        kind: parsed.kind,
        chat_guid: anchor.chat_guid.as_str().to_owned(),
        anchor_message_guid: anchor.message_guid.as_str().to_owned(),
        title,
        confidence_millis: parsed.confidence_millis,
        normalized_time: parsed.time.normalized(&config.reference.timezone),
        evidence_excerpt: excerpt_for_policy(&anchor.excerpt, config.source_excerpts),
        observed_at: anchor.timestamp.as_i64(),
    })
}

pub(crate) fn quiet(
    anchor: &MessageEvidence,
    reason: &'static str,
    source_excerpts: SourceExcerptPolicy,
) -> DetectionOutcome {
    quiet_metadata(anchor, reason, None, source_excerpts)
}

pub(crate) fn quiet_with_provider_diagnostic(
    anchor: &MessageEvidence,
    reason: &'static str,
    provider_diagnostic: String,
    source_excerpts: SourceExcerptPolicy,
) -> DetectionOutcome {
    quiet_metadata(anchor, reason, Some(provider_diagnostic), source_excerpts)
}

fn quiet_metadata(
    anchor: &MessageEvidence,
    reason: &'static str,
    provider_diagnostic: Option<String>,
    source_excerpts: SourceExcerptPolicy,
) -> DetectionOutcome {
    DetectionOutcome::QuietLog(QuietLogDraft {
        chat_guid: anchor.chat_guid.as_str().to_owned(),
        anchor_message_guid: anchor.message_guid.as_str().to_owned(),
        reason: reason.to_owned(),
        excerpt: excerpt_for_policy(&anchor.excerpt, source_excerpts),
        created_at: anchor.timestamp.as_i64(),
        provider_diagnostic,
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
