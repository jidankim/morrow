use morrow_messages::MessageEvidence;
use morrow_storage::CandidateKind;

use crate::types::{CivilDateTime, DetectionConfig};

mod clock;
mod fallback_title;
mod phrases;
mod quantity_list;

use clock::{parse_explicit_datetime, parse_natural_deadline};
use fallback_title::weak_calendar_fallback_title;
use phrases::{
    has_date_like_non_scheduling_context, has_month_date_expression, has_past_temporal_expression,
    has_scheduling_intent, has_scheduling_verb, has_task_scheduling_signal, has_temporal_word,
    has_unsupported_scope, has_weak_calendar_phrase, kind_for, normalized_words, route_reason,
    should_route_explicit_time_to_provider, should_route_to_provider,
};
use quantity_list::should_route_bare_quantity_list;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GateDecision {
    Stop {
        reason: &'static str,
    },
    Candidate(ParsedCandidate),
    ProviderRoute {
        reason: &'static str,
        parser_time: Option<CivilDateTime>,
        fallback: Option<ParsedCandidate>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedCandidate {
    pub kind: CandidateKind,
    pub time: CivilDateTime,
    pub confidence_millis: i64,
    pub title_source: Option<String>,
}

pub(crate) fn classify(message: &MessageEvidence, config: &DetectionConfig) -> GateDecision {
    if message.participant_count > 12 {
        return GateDecision::Stop {
            reason: "deterministic_stop:unsupported_broad_content",
        };
    }
    if message.excerpt.is_empty() || message.evidence_pointer.is_empty() {
        return GateDecision::Stop {
            reason: "deterministic_stop:invalid_evidence",
        };
    }
    let lowered = message.excerpt.to_ascii_lowercase();
    let words = normalized_words(&lowered);
    if has_unsupported_scope(&words) {
        return GateDecision::Stop {
            reason: "deterministic_stop:unsupported_scope",
        };
    }
    if has_date_like_non_scheduling_context(&words) {
        return GateDecision::Stop {
            reason: "deterministic_stop:no_scheduling_signal",
        };
    }

    match parse_explicit_datetime(&message.excerpt) {
        Ok(Some(time)) if time <= config.reference.observed => GateDecision::Stop {
            reason: "deterministic_stop:past_or_invalid_time",
        },
        Ok(Some(_)) if !has_scheduling_intent(&words) => GateDecision::Stop {
            reason: "deterministic_stop:no_scheduling_signal",
        },
        Ok(Some(time)) if is_local_explicit_candidate(&lowered, &words) => {
            GateDecision::Candidate(ParsedCandidate {
                kind: kind_for(&words),
                time,
                confidence_millis: deterministic_confidence(message.tapback_signal),
                title_source: None,
            })
        }
        Ok(Some(time))
            if should_route_explicit_time_to_provider(&words) || is_ambiguous(&lowered) =>
        {
            GateDecision::ProviderRoute {
                reason: route_reason(&words),
                parser_time: Some(time),
                fallback: explicit_provider_fallback(&message.excerpt, &words, time, message),
            }
        }
        Ok(Some(_)) => GateDecision::Stop {
            reason: "deterministic_stop:past_or_invalid_time",
        },
        Ok(None) if has_past_temporal_expression(&words) => GateDecision::Stop {
            reason: "deterministic_stop:past_or_invalid_time",
        },
        Ok(None) if should_route_bare_quantity_list(&message.excerpt, config) => {
            GateDecision::ProviderRoute {
                reason: "parser_provider_route_bare_quantity_list",
                parser_time: None,
                fallback: None,
            }
        }
        Ok(None) if !has_recognized_temporal_expression(&message.excerpt, &words) => {
            GateDecision::Stop {
                reason: "deterministic_stop:no_scheduling_signal",
            }
        }
        Ok(None) if should_route_to_provider(&words) => GateDecision::ProviderRoute {
            reason: route_reason(&words),
            parser_time: None,
            fallback: deadline_fallback(&message.excerpt, &words, config),
        },
        Ok(None) if has_scheduling_verb(&words) => GateDecision::ProviderRoute {
            reason: route_reason(&words),
            parser_time: None,
            fallback: deadline_fallback(&message.excerpt, &words, config),
        },
        Ok(None) => GateDecision::Stop {
            reason: "deterministic_stop:no_scheduling_signal",
        },
        Err(()) => GateDecision::Stop {
            reason: "deterministic_stop:past_or_invalid_time",
        },
    }
}

fn has_recognized_temporal_expression(excerpt: &str, words: &[&str]) -> bool {
    has_temporal_word(words)
        || has_month_date_expression(words)
        || parse_natural_deadline(excerpt)
            .map(|deadline| deadline.is_some())
            .unwrap_or(false)
}

fn is_ambiguous(lowered: &str) -> bool {
    lowered.contains("maybe") || lowered.contains('?')
}

fn is_local_explicit_candidate(lowered: &str, words: &[&str]) -> bool {
    !is_ambiguous(lowered) && !should_route_explicit_time_to_provider(words)
}

fn explicit_provider_fallback(
    excerpt: &str,
    words: &[&str],
    time: CivilDateTime,
    message: &MessageEvidence,
) -> Option<ParsedCandidate> {
    if !has_weak_calendar_phrase(words) {
        return None;
    }
    Some(ParsedCandidate {
        kind: kind_for(words),
        time,
        confidence_millis: deterministic_confidence(message.tapback_signal),
        title_source: weak_calendar_fallback_title(excerpt),
    })
}

fn deterministic_confidence(tapback_signal: bool) -> i64 {
    if tapback_signal {
        900
    } else {
        850
    }
}

fn deadline_fallback(
    excerpt: &str,
    words: &[&str],
    config: &DetectionConfig,
) -> Option<ParsedCandidate> {
    if !has_task_scheduling_signal(words) {
        return None;
    }
    match parse_natural_deadline(excerpt) {
        Ok(Some(time)) if time > config.reference.observed => Some(ParsedCandidate {
            kind: CandidateKind::TaskReminder,
            time,
            confidence_millis: 700,
            title_source: None,
        }),
        Ok(Some(_)) | Ok(None) | Err(()) => None,
    }
}
