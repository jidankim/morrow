use morrow_messages::MessageEvidence;
use morrow_storage::CandidateKind;

use crate::types::{CivilDateTime, DetectionConfig, DetectionError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GateDecision {
    Stop { reason: &'static str },
    Candidate(ParsedCandidate),
    ProviderRoute { parser_time: Option<CivilDateTime> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ParsedCandidate {
    pub kind: CandidateKind,
    pub time: CivilDateTime,
    pub confidence_millis: i64,
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
    if !has_scheduling_verb(&lowered) || !has_temporal_signal(&lowered) {
        return GateDecision::Stop {
            reason: "deterministic_stop:no_scheduling_signal",
        };
    }

    match parse_explicit_datetime(&message.excerpt) {
        Ok(Some(time)) if time <= config.reference.observed => GateDecision::Stop {
            reason: "deterministic_stop:past_or_invalid_time",
        },
        Ok(Some(time)) if is_ambiguous(&lowered) => GateDecision::ProviderRoute {
            parser_time: Some(time),
        },
        Ok(Some(time)) => GateDecision::Candidate(ParsedCandidate {
            kind: kind_for(&lowered),
            time,
            confidence_millis: deterministic_confidence(message.tapback_signal),
        }),
        Ok(None) => GateDecision::ProviderRoute { parser_time: None },
        Err(()) => GateDecision::Stop {
            reason: "deterministic_stop:past_or_invalid_time",
        },
    }
}

const CALENDAR_SCHEDULING_SIGNALS: &[&str] = &[
    "meet",
    "meeting",
    "lunch",
    "dinner",
    "call",
    "appointment",
    "calendar",
    "create",
    "event",
    "schedule",
];

const TASK_SCHEDULING_SIGNALS: &[&str] = &[
    "send", "submit", "finish", "complete", "remind", "reminder", "due", "deadline",
];

fn has_scheduling_verb(lowered: &str) -> bool {
    contains_any(lowered, CALENDAR_SCHEDULING_SIGNALS) || has_task_scheduling_signal(lowered)
}

fn has_task_scheduling_signal(lowered: &str) -> bool {
    contains_any(lowered, TASK_SCHEDULING_SIGNALS)
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    haystack
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|word| needles.contains(&word))
}

fn has_temporal_signal(lowered: &str) -> bool {
    lowered.chars().any(|ch| ch.is_ascii_digit())
        || [
            "today",
            "tomorrow",
            "tonight",
            "monday",
            "tuesday",
            "wednesday",
            "thursday",
            "friday",
            "saturday",
            "sunday",
            "morning",
            "afternoon",
            "evening",
        ]
        .iter()
        .any(|needle| lowered.contains(needle))
}

fn is_ambiguous(lowered: &str) -> bool {
    lowered.contains("maybe") || lowered.contains('?')
}

fn deterministic_confidence(tapback_signal: bool) -> i64 {
    if tapback_signal {
        900
    } else {
        850
    }
}

fn kind_for(lowered: &str) -> CandidateKind {
    if has_task_scheduling_signal(lowered) {
        CandidateKind::TaskReminder
    } else {
        CandidateKind::CalendarEvent
    }
}

pub(crate) fn parse_explicit_datetime(text: &str) -> Result<Option<CivilDateTime>, ()> {
    let words = sanitized_words(text);
    let date = words.iter().find(|word| looks_like_date(word));
    let time = words.iter().find(|word| looks_like_time(word));
    match (date, time) {
        (Some(date), Some(time)) => parse_date_and_time(date, time).map(Some),
        (Some(_), None) | (None, Some(_)) | (None, None) => Ok(None),
    }
}

fn parse_date_and_time(date: &str, time: &str) -> Result<CivilDateTime, ()> {
    let normalized = format!("{date}T{time}:00");
    CivilDateTime::parse_reference(&normalized).map_err(|err| match err {
        DetectionError::InvalidInput {
            field: _,
            reason: _,
        } => (),
    })
}

fn sanitized_words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|word| word.trim_matches(trim_punctuation).to_owned())
        .collect()
}

fn trim_punctuation(ch: char) -> bool {
    matches!(
        ch,
        ',' | '.' | '?' | '!' | ';' | ':' | '(' | ')' | '[' | ']'
    )
}

fn looks_like_date(word: &str) -> bool {
    let mut parts = word.split('-');
    let year = parts.next().is_some_and(|part| part.len() == 4);
    let month = parts.next().is_some_and(|part| part.len() == 2);
    let day = parts.next().is_some_and(|part| part.len() == 2);
    year && month && day && parts.next().is_none()
}

fn looks_like_time(word: &str) -> bool {
    let mut parts = word.split(':');
    let hour = parts.next().is_some_and(|part| !part.is_empty());
    let minute = parts.next().is_some_and(|part| part.len() == 2);
    hour && minute
}
