use std::collections::HashSet;

use serde::{Deserialize, Deserializer};

use super::{
    examples_hash, ListIntakeCategoryRuleRequest, ListIntakeChatScopeRequest,
    ListIntakeOutputPolicyRequest, ListIntakeProfileRequest, ListIntakeQuantityListBoundsRequest,
    ListIntakeThresholdsRequest,
};

const MAX_PROFILE_COUNT: usize = 10;
const MAX_PROFILE_NAME_VISIBLE_CHARS: usize = 48;
const MAX_EXAMPLE_VISIBLE_CHARS: usize = 500;
const MAX_SELECTED_CHAT_IDS: usize = 50;
const MAX_CATEGORY_COUNT: usize = 20;
const MAX_CATEGORY_ID_CHARS: usize = 48;
const MAX_CATEGORY_NAME_VISIBLE_CHARS: usize = 32;
const MAX_CATEGORY_KEYWORD_COUNT: usize = 20;
const MAX_CATEGORY_KEYWORD_VISIBLE_CHARS: usize = 48;
const MAX_EXAMPLES_HASH_CHARS: usize = 64;

pub(in crate::native_bridge::scan) fn deserialize_profiles<'de, D>(
    deserializer: D,
) -> Result<Vec<ListIntakeProfileRequest>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut profiles = Vec::<ListIntakeProfileRequest>::deserialize(deserializer)?;
    validate_profiles(&mut profiles).map_err(serde::de::Error::custom)?;
    Ok(profiles)
}

fn validate_profiles(profiles: &mut [ListIntakeProfileRequest]) -> Result<(), String> {
    validate_max_count("listIntakeProfiles", profiles.len(), MAX_PROFILE_COUNT)?;
    let mut profile_ids = HashSet::with_capacity(profiles.len());
    let mut names = HashSet::with_capacity(profiles.len());
    for profile in profiles {
        validate_profile(profile)?;
        if !profile_ids.insert(profile.profile_id.as_str()) {
            return Err("list-intake profile IDs must be unique".to_owned());
        }
        if !names.insert(profile.name.trim().to_lowercase()) {
            return Err("list-intake profile names must be unique".to_owned());
        }
    }
    Ok(())
}

fn validate_profile(profile: &mut ListIntakeProfileRequest) -> Result<(), String> {
    ensure(
        has_profile_id_shape(&profile.profile_id),
        "profileId must match list-intake-[a-z0-9-]{8,48}",
    )?;
    validate_visible_text("name", &profile.name, 1, MAX_PROFILE_NAME_VISIBLE_CHARS)?;
    validate_examples("positiveExamples", &profile.positive_examples, 1, 20)?;
    validate_examples("negativeExamples", &profile.negative_examples, 0, 20)?;
    validate_categories(&profile.category_rules)?;
    validate_chat_scope(&profile.chat_scope)?;
    ensure(profile.grouping.chat, "grouping.chat must be true")?;
    validate_output_policy(profile)?;
    validate_quantity_list_bounds(&profile.quantity_list_bounds)?;
    validate_thresholds(&profile.thresholds)?;
    validate_visible_text(
        "examplesHash",
        &profile.examples_hash,
        1,
        MAX_EXAMPLES_HASH_CHARS,
    )?;
    profile.examples_hash = examples_hash::computed_examples_hash(profile)?;
    Ok(())
}

fn validate_examples(
    field: &'static str,
    examples: &[String],
    min_count: usize,
    max_count: usize,
) -> Result<(), String> {
    validate_count(field, examples.len(), min_count, max_count)?;
    examples
        .iter()
        .try_for_each(|example| validate_visible_text(field, example, 1, MAX_EXAMPLE_VISIBLE_CHARS))
}

fn validate_categories(categories: &[ListIntakeCategoryRuleRequest]) -> Result<(), String> {
    validate_max_count("categoryRules", categories.len(), MAX_CATEGORY_COUNT)?;
    let mut category_ids = HashSet::with_capacity(categories.len());
    for category in categories {
        ensure(
            has_category_id_shape(&category.category_id),
            "categoryId must match [a-z0-9][a-z0-9-]{0,47}",
        )?;
        ensure(
            category_ids.insert(category.category_id.as_str()),
            "category IDs must be unique",
        )?;
        validate_visible_text(
            "category displayName",
            &category.display_name,
            1,
            MAX_CATEGORY_NAME_VISIBLE_CHARS,
        )?;
        validate_count(
            "category keywords",
            category.keywords.len(),
            0,
            MAX_CATEGORY_KEYWORD_COUNT,
        )?;
        category.keywords.iter().try_for_each(|keyword| {
            validate_visible_text(
                "category keyword",
                keyword,
                1,
                MAX_CATEGORY_KEYWORD_VISIBLE_CHARS,
            )
        })?;
    }
    Ok(())
}

fn validate_chat_scope(scope: &ListIntakeChatScopeRequest) -> Result<(), String> {
    match scope {
        ListIntakeChatScopeRequest::AllSelectedChats {} => Ok(()),
        ListIntakeChatScopeRequest::SelectedChatIds { selected_chat_ids } => {
            validate_count(
                "selectedChatIds",
                selected_chat_ids.len(),
                1,
                MAX_SELECTED_CHAT_IDS,
            )?;
            selected_chat_ids.iter().try_for_each(|chat_id| {
                ensure(
                    has_opaque_chat_id_shape(chat_id),
                    "selectedChatIds must be opaque Messages chat IDs",
                )
            })
        }
    }
}

fn validate_output_policy(profile: &ListIntakeProfileRequest) -> Result<(), String> {
    match profile.output_policy {
        ListIntakeOutputPolicyRequest::AggregateOnly => ensure(
            profile.digest_reminder.is_none(),
            "aggregateOnly profiles must not include digestReminder",
        ),
        ListIntakeOutputPolicyRequest::DailyDigestReminder => {
            let Some(digest) = &profile.digest_reminder else {
                return Err("dailyDigestReminder profiles require digestReminder".to_owned());
            };
            ensure(
                digest.date_offset_days == 1,
                "digestReminder.dateOffsetDays must be 1",
            )
        }
    }
}

fn validate_quantity_list_bounds(
    bounds: &ListIntakeQuantityListBoundsRequest,
) -> Result<(), String> {
    ensure(
        bounds.min_items == 1
            && bounds.max_items == 20
            && bounds.min_quantity == 1
            && bounds.max_quantity == 999
            && bounds.max_item_name_visible_chars == 80
            && bounds.max_unit_visible_chars == 24
            && bounds.uncategorized_category_id == "uncategorized",
        "quantityListBounds must match locked v2 defaults",
    )
}

fn validate_thresholds(thresholds: &ListIntakeThresholdsRequest) -> Result<(), String> {
    ensure(
        thresholds.auto_aggregate_threshold_millis == 850
            && thresholds.review_threshold_millis == 550,
        "thresholds must match locked v2 defaults",
    )
}

fn validate_count(
    field: &'static str,
    count: usize,
    min_count: usize,
    max_count: usize,
) -> Result<(), String> {
    if (min_count..=max_count).contains(&count) {
        Ok(())
    } else {
        Err(format!(
            "{field} count must be between {min_count} and {max_count}"
        ))
    }
}

fn validate_max_count(field: &'static str, count: usize, max_count: usize) -> Result<(), String> {
    if count <= max_count {
        Ok(())
    } else {
        Err(format!("{field} count must be at most {max_count}"))
    }
}

fn validate_visible_text(
    field: &'static str,
    value: &str,
    min_chars: usize,
    max_chars: usize,
) -> Result<(), String> {
    let visible_chars = value.trim().chars().count();
    if (min_chars..=max_chars).contains(&visible_chars) {
        Ok(())
    } else {
        Err(format!(
            "{field} visible length must be between {min_chars} and {max_chars}"
        ))
    }
}

fn has_profile_id_shape(value: &str) -> bool {
    let Some(suffix) = value.strip_prefix("list-intake-") else {
        return false;
    };
    (8..=48).contains(&suffix.len()) && suffix.bytes().all(is_lower_digit_or_hyphen)
}

fn has_category_id_shape(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    value.len() <= MAX_CATEGORY_ID_CHARS
        && (first.is_ascii_lowercase() || first.is_ascii_digit())
        && bytes.all(is_lower_digit_or_hyphen)
}

fn has_opaque_chat_id_shape(value: &str) -> bool {
    let Some(suffix) = value.strip_prefix("messages-chat-") else {
        return false;
    };
    suffix.len() == 32 && suffix.bytes().all(is_lower_hex_digit)
}

fn ensure(condition: bool, message: &'static str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}

fn is_lower_digit_or_hyphen(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
}

fn is_lower_hex_digit(byte: u8) -> bool {
    byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
}
