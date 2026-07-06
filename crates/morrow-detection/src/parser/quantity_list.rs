use crate::types::{DetectionConfig, ListReminderRoutingMode};

pub(crate) fn should_route_bare_quantity_list(excerpt: &str, config: &DetectionConfig) -> bool {
    config.profile.enabled
        && matches!(
            config.profile.routing_mode,
            ListReminderRoutingMode::ProfileBareQuantityLists
        )
        && looks_like_quantity_item_list(excerpt)
}

fn looks_like_quantity_item_list(excerpt: &str) -> bool {
    let items = excerpt
        .split([',', ';', '\n'])
        .map(str::trim)
        .collect::<Vec<_>>();
    (2..=20).contains(&items.len()) && items.iter().all(|item| looks_like_quantity_item(item))
}

fn looks_like_quantity_item(item: &str) -> bool {
    if item.is_empty() || item.contains(['@', ':', '/', '\\', '$', '-']) {
        return false;
    }
    let digit_len = item.bytes().take_while(u8::is_ascii_digit).count();
    if digit_len == 0 || digit_len > 3 {
        return false;
    }
    let (quantity, rest) = item.split_at(digit_len);
    if quantity.starts_with('0') || !rest.starts_with(char::is_whitespace) {
        return false;
    }
    let Ok(quantity) = quantity.parse::<u16>() else {
        return false;
    };
    if !(1..=999).contains(&quantity) {
        return false;
    }
    let words = rest.split_whitespace().collect::<Vec<_>>();
    (1..=5).contains(&words.len())
        && words
            .iter()
            .all(|word| is_visible_item_word(word) && !is_ampm_time_marker(word))
}

fn is_visible_item_word(word: &str) -> bool {
    word.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
        && word.bytes().any(|byte| byte.is_ascii_alphabetic())
        && !word.contains(['@', ':', '/', '\\', '$', ','])
        && !looks_like_bare_domain_word(word)
}

fn looks_like_bare_domain_word(word: &str) -> bool {
    let candidate = word.trim_end_matches(['.', '!', '?']);
    let mut labels = candidate.split('.');
    let Some(first_label) = labels.next() else {
        return false;
    };
    if !is_domain_label(first_label) {
        return false;
    }

    let mut label_count = 1usize;
    let mut final_label = first_label;
    for label in labels {
        if !is_domain_label(label) {
            return false;
        }
        label_count += 1;
        final_label = label;
    }

    label_count >= 2 && is_domain_suffix(final_label)
}

fn is_domain_label(label: &str) -> bool {
    !label.is_empty() && label.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

fn is_domain_suffix(label: &str) -> bool {
    label.len() >= 2 && label.bytes().all(|byte| byte.is_ascii_alphabetic())
}

fn is_ampm_time_marker(word: &str) -> bool {
    word.eq_ignore_ascii_case("am")
        || word.eq_ignore_ascii_case("pm")
        || word.eq_ignore_ascii_case("a.m.")
        || word.eq_ignore_ascii_case("p.m.")
        || word.eq_ignore_ascii_case("a.m")
        || word.eq_ignore_ascii_case("p.m")
}
