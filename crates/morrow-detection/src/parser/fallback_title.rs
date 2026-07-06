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
    for marker in TIME_MARKERS {
        if let Some(index) = lowered.find(marker) {
            boundary = boundary.min(index);
        }
    }
    boundary
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

const TIME_MARKERS: &[&str] = &[
    " on ",
    " at ",
    " from ",
    " until ",
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
