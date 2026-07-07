pub(super) fn weak_calendar_fallback_title(excerpt: &str) -> Option<String> {
    let trimmed = excerpt.trim();
    let subject = subject_after_context_label(trimmed);
    let title = subject_before_time_marker(subject)
        .trim()
        .trim_matches(|ch| matches!(ch, ':' | '-' | ',' | '.' | ';'))
        .trim();

    if title.is_empty() || title == trimmed {
        None
    } else {
        Some(bounded_title(title, 120))
    }
}

fn subject_after_context_label(excerpt: &str) -> &str {
    let time_boundary = time_marker_boundary(excerpt);
    match excerpt.find(':') {
        Some(label_boundary) if label_boundary < time_boundary => {
            excerpt[label_boundary + 1..].trim()
        }
        Some(_) | None => excerpt,
    }
}

fn subject_before_time_marker(excerpt: &str) -> &str {
    &excerpt[..time_marker_boundary(excerpt)]
}

fn time_marker_boundary(excerpt: &str) -> usize {
    let lowered = excerpt.to_ascii_lowercase();
    let mut boundary = excerpt.len();
    for marker in DIRECT_TIME_MARKERS {
        if let Some(index) = lowered.find(marker) {
            boundary = boundary.min(index);
        }
    }
    for marker in CONTEXTUAL_TIME_MARKERS {
        boundary = boundary.min(contextual_marker_boundary(&lowered, marker));
    }
    boundary
}

fn contextual_marker_boundary(lowered: &str, marker: &str) -> usize {
    let mut offset = 0;
    while let Some(relative_index) = lowered[offset..].find(marker) {
        let index = offset + relative_index;
        let after_marker = index + marker.len();
        if starts_time_expression(&lowered[after_marker..]) {
            return index;
        }
        offset = after_marker;
    }
    lowered.len()
}

fn starts_time_expression(raw: &str) -> bool {
    let tail = raw.trim_start();
    starts_year_date(tail)
        || starts_clock_time(tail)
        || starts_month_word(tail)
        || starts_day_month(tail)
}

fn starts_year_date(tail: &str) -> bool {
    let bytes = tail.as_bytes();
    let Some((year, rest)) = bytes.split_at_checked(4) else {
        return false;
    };
    digits(year) && rest.first().is_some_and(|byte| matches!(byte, b'-' | b'/'))
}

fn starts_clock_time(tail: &str) -> bool {
    let bytes = tail.as_bytes();
    if !bytes.first().is_some_and(u8::is_ascii_digit) {
        return false;
    }
    let digit_count = bytes
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digit_count == 0 || digit_count > 2 {
        return false;
    }
    let rest = &tail[digit_count..];
    rest.starts_with(':') || rest.starts_with(" am") || rest.starts_with(" pm")
}

fn starts_month_word(tail: &str) -> bool {
    MONTH_WORDS.iter().any(|month| starts_word(tail, month))
}

fn starts_day_month(tail: &str) -> bool {
    let bytes = tail.as_bytes();
    let digit_count = bytes
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digit_count == 0 || digit_count > 2 {
        return false;
    }
    let rest = tail[digit_count..].trim_start();
    starts_month_word(rest)
}

fn starts_word(tail: &str, word: &str) -> bool {
    tail.strip_prefix(word).is_some_and(|rest| {
        rest.as_bytes()
            .first()
            .is_none_or(|byte| !byte.is_ascii_alphanumeric())
    })
}

fn digits(bytes: &[u8]) -> bool {
    bytes.iter().all(u8::is_ascii_digit)
}

fn bounded_title(title: &str, max_bytes: usize) -> String {
    let mut output = String::new();
    for ch in title.chars() {
        let next_len = output.len() + ch.len_utf8();
        if next_len > max_bytes {
            break;
        }
        output.push(ch);
    }
    output
}

const CONTEXTUAL_TIME_MARKERS: &[&str] = &[" on ", " at ", " from ", " until "];

const DIRECT_TIME_MARKERS: &[&str] = &[
    " today",
    " tomorrow",
    " tonight",
    " monday",
    " tuesday",
    " wednesday",
    " thursday",
    " friday",
    " saturday",
    " sunday",
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

#[cfg(test)]
mod tests {
    use super::weak_calendar_fallback_title;

    #[test]
    fn one_on_one_title_keeps_internal_on() {
        // Given
        let excerpt =
            "Morrow QA live receipt test: 1 on 1 meeting on 2026-07-17 at 9:30 AM for 30 minutes";

        // When
        let title = weak_calendar_fallback_title(excerpt);

        // Then
        assert_eq!(title.as_deref(), Some("1 on 1 meeting"));
    }

    #[test]
    fn coffee_sync_title_stops_at_date_marker() {
        // Given
        let excerpt =
            "Morrow QA live receipt test: coffee sync on 2026-07-17 at 9:30 AM for 30 minutes";

        // When
        let title = weak_calendar_fallback_title(excerpt);

        // Then
        assert_eq!(title.as_deref(), Some("coffee sync"));
    }
}
