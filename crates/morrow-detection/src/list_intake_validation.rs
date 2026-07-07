use morrow_messages::MessageEvidence;
use serde::Deserialize;
use time::OffsetDateTime;
use time_tz::{timezones, OffsetDateTimeExt, PrimitiveDateTimeExt, TimeZone};

use super::{
    ListIntakeConfidenceTier, ListIntakeLocalDayWindow, ListIntakeProfile,
    ListIntakeValidationError, ValidatedListIntakeExtraction, ValidatedListIntakeItem,
    LIST_INTAKE_PROVIDER_SCHEMA_VERSION, LIST_INTAKE_UNCATEGORIZED_CATEGORY_ID,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderOutput {
    matched: bool,
    confidence_millis: u16,
    items: Vec<ProviderOutputItem>,
    #[serde(default)]
    rejection_reason: Option<ProviderRejectionReason>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ProviderRejectionReason {
    NotAList,
    SchedulingIntent,
    Ambiguous,
    Unsupported,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderOutputItem {
    name: String,
    quantity: u16,
    #[serde(default)]
    unit: Option<String>,
    #[serde(rename = "categoryId")]
    category_id: String,
    #[serde(default, rename = "evidenceText")]
    evidence_text: Option<String>,
}

pub fn validate_list_intake_provider_output(
    profile: &ListIntakeProfile,
    evidence: &MessageEvidence,
    reference_timezone: &str,
    provider_output: &str,
) -> Result<ValidatedListIntakeExtraction, ListIntakeValidationError> {
    let output: ProviderOutput = serde_json::from_str(provider_output)
        .map_err(|_| ListIntakeValidationError::MalformedJson)?;
    if output.confidence_millis > 1000 {
        return Err(invalid_output("confidence_millis was outside 0..1000"));
    }
    if !output.matched {
        let _ = output.rejection_reason;
        return validated_extraction(
            profile,
            evidence,
            reference_timezone,
            Vec::new(),
            output.confidence_millis,
            false,
        );
    }
    if output.items.is_empty() || output.items.len() > 20 {
        return Err(invalid_output("matched output must include 1 to 20 items"));
    }
    let mut lowered_to_review = false;
    let items = output
        .items
        .into_iter()
        .map(|item| validated_item(profile, evidence, item, &mut lowered_to_review))
        .collect::<Result<Vec<_>, _>>()?;
    validated_extraction(
        profile,
        evidence,
        reference_timezone,
        items,
        output.confidence_millis,
        lowered_to_review,
    )
}

fn validated_item(
    profile: &ListIntakeProfile,
    evidence: &MessageEvidence,
    item: ProviderOutputItem,
    lowered_to_review: &mut bool,
) -> Result<ValidatedListIntakeItem, ListIntakeValidationError> {
    let name = normalized_visible_text(&item.name, 80, "item name was invalid")?;
    if item.quantity == 0 || item.quantity > 999 {
        return Err(invalid_output("item quantity was invalid"));
    }
    let unit = item
        .unit
        .map(|unit| normalized_visible_text(&unit, 24, "item unit was invalid"))
        .transpose()?;
    let evidence_text = item
        .evidence_text
        .map(|text| normalized_visible_text(&text, 120, "item evidence text was invalid"))
        .transpose()?;
    if !item_is_grounded(&name, evidence_text.as_deref(), &evidence.excerpt) {
        return Err(invalid_output("item was not grounded in evidence"));
    }
    let category_id = mapped_category_id(profile, &item.category_id, &name, lowered_to_review);
    Ok(ValidatedListIntakeItem {
        name,
        quantity: item.quantity,
        unit,
        category_id,
        evidence_text,
    })
}

fn validated_extraction(
    profile: &ListIntakeProfile,
    evidence: &MessageEvidence,
    reference_timezone: &str,
    items: Vec<ValidatedListIntakeItem>,
    confidence_millis: u16,
    lowered_to_review: bool,
) -> Result<ValidatedListIntakeExtraction, ListIntakeValidationError> {
    Ok(ValidatedListIntakeExtraction {
        profile_id: profile.profile_id.clone(),
        profile_version: profile.profile_version.clone(),
        examples_hash: profile.examples_hash.clone(),
        evidence_pointer: evidence.evidence_pointer.clone(),
        message_guid: evidence.message_guid.as_str().to_owned(),
        items,
        window: local_day_window(evidence, reference_timezone)?,
        confidence_tier: confidence_tier(confidence_millis, lowered_to_review),
        confidence_millis,
        provider_prompt_version: profile.provider_prompt_version.clone(),
        provider_schema_version: LIST_INTAKE_PROVIDER_SCHEMA_VERSION.to_owned(),
    })
}

fn normalized_visible_text(
    value: &str,
    max_chars: usize,
    reason: &'static str,
) -> Result<String, ListIntakeValidationError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.chars().count() > max_chars
        || trimmed
            .chars()
            .any(|ch| ch.is_control() || matches!(ch, '[' | ']' | '{' | '}' | '/' | '\\' | '@'))
    {
        return Err(invalid_output(reason));
    }
    Ok(trimmed.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn mapped_category_id(
    profile: &ListIntakeProfile,
    provider_category_id: &str,
    item_name: &str,
    lowered_to_review: &mut bool,
) -> String {
    if profile
        .category_rules
        .iter()
        .any(|rule| rule.category_id == provider_category_id)
    {
        return provider_category_id.to_owned();
    }
    if provider_category_id == LIST_INTAKE_UNCATEGORIZED_CATEGORY_ID {
        return keyword_category(profile, item_name)
            .unwrap_or_else(|| LIST_INTAKE_UNCATEGORIZED_CATEGORY_ID.to_owned());
    }
    *lowered_to_review = true;
    LIST_INTAKE_UNCATEGORIZED_CATEGORY_ID.to_owned()
}

fn keyword_category(profile: &ListIntakeProfile, item_name: &str) -> Option<String> {
    let rules = profile
        .category_rules
        .iter()
        .map(|rule| morrow_storage::ListIntakeCategoryRule {
            category_id: rule.category_id.clone(),
            keywords: rule.keywords.clone(),
        })
        .collect::<Vec<_>>();
    let category = morrow_storage::assign_list_intake_category(item_name, &rules);
    (category != LIST_INTAKE_UNCATEGORIZED_CATEGORY_ID).then_some(category)
}

fn confidence_tier(confidence_millis: u16, lowered_to_review: bool) -> ListIntakeConfidenceTier {
    if confidence_millis < 550 {
        ListIntakeConfidenceTier::Low
    } else if confidence_millis >= 850 && !lowered_to_review {
        ListIntakeConfidenceTier::AutoAggregate
    } else {
        ListIntakeConfidenceTier::Review
    }
}

fn item_is_grounded(name: &str, evidence_text: Option<&str>, excerpt: &str) -> bool {
    let excerpt = excerpt.to_ascii_lowercase();
    let name = name.to_ascii_lowercase();
    match evidence_text {
        Some(text) => {
            let text = text.to_ascii_lowercase();
            excerpt.contains(&text) && text.contains(&name)
        }
        None => excerpt.contains(&name),
    }
}

fn local_day_window(
    evidence: &MessageEvidence,
    timezone: &str,
) -> Result<ListIntakeLocalDayWindow, ListIntakeValidationError> {
    let timezone = timezones::get_by_name(timezone)
        .ok_or_else(|| invalid_output("reference timezone was unsupported"))?;
    let utc = OffsetDateTime::from_unix_timestamp(evidence.timestamp.as_i64())
        .map_err(|_| invalid_output("message timestamp was invalid"))?;
    let local = utc.to_timezone(timezone);
    let local_midnight = local.date().midnight();
    let window_start = local_midnight
        .assume_timezone(timezone)
        .take_first()
        .ok_or_else(|| invalid_output("window start was invalid"))?;
    Ok(ListIntakeLocalDayWindow {
        window_local_date: local.date().to_string(),
        window_timezone: timezone.name().to_owned(),
        window_start_unix_seconds: window_start.unix_timestamp(),
    })
}

const fn invalid_output(reason: &'static str) -> ListIntakeValidationError {
    ListIntakeValidationError::InvalidProviderOutput { reason }
}
