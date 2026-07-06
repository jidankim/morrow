use morrow_messages::MessageEvidence;
use morrow_storage::CandidateKind;

use crate::types::{CivilDateTime, DetectionConfig};

mod clock;
mod fallback_title;
mod quantity_list;

use clock::{parse_explicit_datetime, parse_natural_deadline};
use fallback_title::weak_calendar_fallback_title;
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

    match parse_explicit_datetime(&message.excerpt) {
        Ok(Some(time)) if time <= config.reference.observed => GateDecision::Stop {
            reason: "deterministic_stop:past_or_invalid_time",
        },
        Ok(Some(_)) if !has_scheduling_intent(&words) => GateDecision::Stop {
            reason: "deterministic_stop:no_scheduling_signal",
        },
        Ok(Some(time)) if is_ambiguous(&lowered) && !has_weak_calendar_phrase(&words) => {
            GateDecision::ProviderRoute {
                reason: "parser_provider_route_ambiguous_calendar",
                parser_time: Some(time),
                fallback: None,
            }
        }
        Ok(Some(time)) if has_weak_calendar_phrase(&words) => GateDecision::ProviderRoute {
            reason: "parser_provider_route_weak_calendar",
            parser_time: Some(time),
            fallback: Some(ParsedCandidate {
                kind: kind_for(&words),
                time,
                confidence_millis: deterministic_confidence(message.tapback_signal),
                title_source: weak_calendar_fallback_title(&message.excerpt),
            }),
        },
        Ok(Some(time)) if time > config.reference.observed => {
            GateDecision::Candidate(ParsedCandidate {
                kind: kind_for(&words),
                time,
                confidence_millis: deterministic_confidence(message.tapback_signal),
                title_source: None,
            })
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
            reason: "parser_provider_route_ambiguous_calendar",
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
    "sync",
];

const TASK_SCHEDULING_SIGNALS: &[&str] = &[
    "send", "submit", "finish", "complete", "remind", "reminder", "due", "deadline",
];

const TEMPORAL_WORDS: &[&str] = &[
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
];

const MONTH_WORDS: &[&str] = &[
    "january",
    "jan",
    "february",
    "feb",
    "march",
    "mar",
    "april",
    "apr",
    "may",
    "june",
    "jun",
    "july",
    "jul",
    "august",
    "aug",
    "september",
    "sep",
    "october",
    "oct",
    "november",
    "nov",
    "december",
    "dec",
];

fn has_scheduling_intent(words: &[&str]) -> bool {
    has_scheduling_verb(words) || has_weak_calendar_phrase(words)
}

fn has_scheduling_verb(words: &[&str]) -> bool {
    contains_any(words, CALENDAR_SCHEDULING_SIGNALS) || has_task_scheduling_signal(words)
}

fn has_task_scheduling_signal(words: &[&str]) -> bool {
    contains_any(words, TASK_SCHEDULING_SIGNALS)
        || contains_phrase(words, &["follow", "up"])
        || contains_phrase(words, &["review", "by"])
}

fn contains_any(words: &[&str], needles: &[&str]) -> bool {
    words.iter().any(|word| needles.contains(word))
}

fn has_weak_calendar_phrase(words: &[&str]) -> bool {
    contains_phrase(words, &["catch", "up"])
        || contains_any(words, &["coffee", "sync"])
        || contains_phrase(words, &["touch", "base"])
}

fn contains_phrase(words: &[&str], phrase: &[&str]) -> bool {
    words.windows(phrase.len()).any(|window| window == phrase)
}

fn has_recognized_temporal_expression(excerpt: &str, words: &[&str]) -> bool {
    contains_any(words, TEMPORAL_WORDS)
        || has_month_date_expression(words)
        || parse_natural_deadline(excerpt)
            .map(|deadline| deadline.is_some())
            .unwrap_or(false)
}

fn has_month_date_expression(words: &[&str]) -> bool {
    words.windows(2).any(|window| {
        let [first, second] = window else {
            return false;
        };
        (MONTH_WORDS.contains(first) && is_calendar_day(second))
            || (is_calendar_day(first) && MONTH_WORDS.contains(second))
    })
}

fn is_calendar_day(word: &str) -> bool {
    word.parse::<u8>().is_ok_and(|day| (1..=31).contains(&day))
}

fn has_past_temporal_expression(words: &[&str]) -> bool {
    contains_any(words, &["yesterday"])
}

fn should_route_to_provider(words: &[&str]) -> bool {
    has_weak_calendar_phrase(words) || has_task_scheduling_signal(words)
}

fn route_reason(words: &[&str]) -> &'static str {
    if has_task_scheduling_signal(words) {
        "parser_provider_route_task_deadline"
    } else {
        "parser_provider_route_weak_calendar"
    }
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

fn kind_for(words: &[&str]) -> CandidateKind {
    if has_task_scheduling_signal(words) {
        CandidateKind::TaskReminder
    } else {
        CandidateKind::CalendarEvent
    }
}

fn normalized_words(text: &str) -> Vec<&str> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect()
}
