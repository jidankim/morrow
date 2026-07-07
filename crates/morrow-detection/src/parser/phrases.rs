use morrow_storage::CandidateKind;

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
    "send", "submit", "finish", "complete", "remind", "reminder", "due", "deadline", "owe",
];

const PROVIDER_CALENDAR_SIGNALS: &[&str] = &["book", "hold", "lock", "pencil", "slot"];

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

pub(super) fn has_scheduling_intent(words: &[&str]) -> bool {
    has_scheduling_verb(words) || has_weak_calendar_phrase(words)
}

pub(super) fn has_scheduling_verb(words: &[&str]) -> bool {
    contains_any(words, CALENDAR_SCHEDULING_SIGNALS) || has_task_scheduling_signal(words)
}

pub(super) fn has_task_scheduling_signal(words: &[&str]) -> bool {
    contains_any(words, TASK_SCHEDULING_SIGNALS)
        || contains_phrase(words, &["follow", "up"])
        || contains_phrase(words, &["review", "by"])
}

pub(super) fn has_unsupported_scope(words: &[&str]) -> bool {
    contains_any(words, &["screenshot", "itinerary", "portal"])
        || contains_phrase(words, &["every", "date"])
        || contains_phrase(words, &["whole", "travel"])
        || contains_phrase(words, &["other", "conversations"])
        || contains_phrase(words, &["last", "year", "s"])
        || contains_phrase(words, &["last", "years"])
}

pub(super) fn has_date_like_non_scheduling_context(words: &[&str]) -> bool {
    has_explicit_rejection(words)
        || contains_phrase(words, &["lunch", "menu"])
        || contains_phrase(words, &["calendar", "rendering"])
        || contains_phrase(words, &["july", "build"])
        || contains_phrase(words, &["schedule", "view"])
        || contains_phrase(words, &["call", "stack"])
        || contains_phrase(words, &["due", "date", "parser"])
}

pub(super) fn has_weak_calendar_phrase(words: &[&str]) -> bool {
    contains_phrase(words, &["catch", "up"])
        || contains_phrase(words, &["1", "on", "1"])
        || contains_phrase(words, &["one", "on", "one"])
        || contains_any(words, &["coffee", "sync"])
        || contains_phrase(words, &["touch", "base"])
}

pub(super) fn has_temporal_word(words: &[&str]) -> bool {
    contains_any(words, TEMPORAL_WORDS)
}

pub(super) fn has_month_date_expression(words: &[&str]) -> bool {
    words.windows(2).any(|window| {
        let [first, second] = window else {
            return false;
        };
        (MONTH_WORDS.contains(first) && is_calendar_day(second))
            || (is_calendar_day(first) && MONTH_WORDS.contains(second))
    })
}

pub(super) fn has_past_temporal_expression(words: &[&str]) -> bool {
    contains_any(words, &["yesterday"])
        || (contains_from_temporal_word(words) && !has_lifecycle_update_signal(words))
}

pub(super) fn should_route_to_provider(words: &[&str]) -> bool {
    should_route_explicit_time_to_provider(words) || has_task_scheduling_signal(words)
}

pub(super) fn should_route_explicit_time_to_provider(words: &[&str]) -> bool {
    has_weak_calendar_phrase(words) || has_natural_calendar_signal(words)
}

pub(super) fn route_reason(words: &[&str]) -> &'static str {
    if has_lifecycle_update_signal(words) {
        "parser_provider_route_lifecycle_update"
    } else if has_task_scheduling_signal(words) {
        "parser_provider_route_task_deadline"
    } else if has_weak_calendar_phrase(words) {
        "parser_provider_route_weak_calendar"
    } else {
        "parser_provider_route_ambiguous_calendar"
    }
}

pub(super) fn kind_for(words: &[&str]) -> CandidateKind {
    if has_task_scheduling_signal(words) {
        CandidateKind::TaskReminder
    } else {
        CandidateKind::CalendarEvent
    }
}

pub(super) fn normalized_words(text: &str) -> Vec<&str> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect()
}

fn contains_any(words: &[&str], needles: &[&str]) -> bool {
    words.iter().any(|word| needles.contains(word))
}

fn contains_phrase(words: &[&str], phrase: &[&str]) -> bool {
    words.windows(phrase.len()).any(|window| window == phrase)
}

fn has_natural_calendar_signal(words: &[&str]) -> bool {
    contains_any(words, PROVIDER_CALENDAR_SIGNALS)
        || contains_phrase(words, &["let", "s", "do"])
        || contains_phrase(words, &["can", "see", "me"])
}

fn has_explicit_rejection(words: &[&str]) -> bool {
    contains_phrase(words, &["not", "schedule"])
        || contains_phrase(words, &["no", "reminder"])
        || contains_phrase(words, &["not", "a", "calendar"])
}

fn contains_from_temporal_word(words: &[&str]) -> bool {
    words.windows(2).any(|window| {
        let [first, second] = window else {
            return false;
        };
        *first == "from" && TEMPORAL_WORDS.contains(second)
    })
}

pub(super) fn has_lifecycle_update_signal(words: &[&str]) -> bool {
    contains_any(words, &["moved", "move", "instead", "reschedule"])
}

fn is_calendar_day(word: &str) -> bool {
    word.parse::<u8>().is_ok_and(|day| (1..=31).contains(&day))
}
