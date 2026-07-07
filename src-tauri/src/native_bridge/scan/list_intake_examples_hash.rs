use serde_json::Value;

use morrow_detection::LIST_INTAKE_PROVIDER_SCHEMA_VERSION;

use super::{
    ListIntakeChatScopeRequest, ListIntakeOutputPolicyRequest, ListIntakeProfileRequest,
    ListIntakeProfileVersionRequest, ListIntakeProviderPromptVersionRequest,
};

const FNV_OFFSET: u32 = 2_166_136_261;
const FNV_PRIME: u32 = 16_777_619;

pub(super) fn computed_examples_hash(profile: &ListIntakeProfileRequest) -> Result<String, String> {
    let identity = examples_identity(profile)?;
    Ok(format!("list-intake-{}", fnv1a32(&identity)))
}

fn examples_identity(profile: &ListIntakeProfileRequest) -> Result<String, String> {
    let mut fields = vec![
        field("profileVersion", profile_version(profile.profile_version))?,
        field(
            "providerPromptVersion",
            provider_prompt_version(profile.provider_prompt_version),
        )?,
        field(
            "providerSchemaVersion",
            quote(LIST_INTAKE_PROVIDER_SCHEMA_VERSION)?,
        )?,
        field("positiveExamples", strings(&profile.positive_examples)?)?,
        field("negativeExamples", strings(&profile.negative_examples)?)?,
        field("categoryRules", category_rules(profile)?)?,
        field("chatScope", chat_scope(&profile.chat_scope)?)?,
        field("thresholds", thresholds(profile))?,
        field("outputPolicy", output_policy(profile.output_policy))?,
    ];
    if let Some(digest_reminder) = &profile.digest_reminder {
        fields.push(field(
            "digestReminder",
            format!(
                "{{\"dueTimeLocal\":\"09:00\",\"dateOffsetDays\":{},\"outputPolicyVersion\":\"list-intake-digest-v1\"}}",
                digest_reminder.date_offset_days
            ),
        )?);
    }
    fields.push(field("quantityListBounds", quantity_list_bounds(profile))?);
    Ok(format!("{{{}}}", fields.join(",")))
}

fn category_rules(profile: &ListIntakeProfileRequest) -> Result<String, String> {
    let rules = profile
        .category_rules
        .iter()
        .map(|rule| {
            Ok(format!(
                "{{\"categoryId\":{},\"displayName\":{},\"keywords\":{}}}",
                quote(&rule.category_id)?,
                quote(&rule.display_name)?,
                strings(&rule.keywords)?
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(format!("[{}]", rules.join(",")))
}

fn chat_scope(scope: &ListIntakeChatScopeRequest) -> Result<String, String> {
    match scope {
        ListIntakeChatScopeRequest::AllSelectedChats {} => {
            Ok("{\"mode\":\"allSelectedChats\"}".to_owned())
        }
        ListIntakeChatScopeRequest::SelectedChatIds { selected_chat_ids } => Ok(format!(
            "{{\"mode\":\"selectedChatIds\",\"selectedChatIds\":{}}}",
            strings(selected_chat_ids)?
        )),
    }
}

fn thresholds(profile: &ListIntakeProfileRequest) -> String {
    format!(
        "{{\"autoAggregateThresholdMillis\":{},\"reviewThresholdMillis\":{}}}",
        profile.thresholds.auto_aggregate_threshold_millis,
        profile.thresholds.review_threshold_millis
    )
}

fn quantity_list_bounds(profile: &ListIntakeProfileRequest) -> String {
    let bounds = &profile.quantity_list_bounds;
    format!(
        "{{\"minItems\":{},\"maxItems\":{},\"minQuantity\":{},\"maxQuantity\":{},\"maxItemNameVisibleChars\":{},\"maxUnitVisibleChars\":{},\"uncategorizedCategoryId\":\"{}\"}}",
        bounds.min_items,
        bounds.max_items,
        bounds.min_quantity,
        bounds.max_quantity,
        bounds.max_item_name_visible_chars,
        bounds.max_unit_visible_chars,
        bounds.uncategorized_category_id
    )
}

fn field(name: &str, value: impl Into<String>) -> Result<String, String> {
    Ok(format!("{}:{}", quote(name)?, value.into()))
}

fn strings(values: &[String]) -> Result<String, String> {
    let quoted = values
        .iter()
        .map(|value| quote(value))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(format!("[{}]", quoted.join(",")))
}

fn quote(value: &str) -> Result<String, String> {
    serde_json::to_string(&Value::String(value.to_owned())).map_err(|error| error.to_string())
}

fn fnv1a32(input: &str) -> String {
    let mut hash = FNV_OFFSET;
    for ch in input.chars() {
        hash ^= u32::from(ch);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:08x}")
}

const fn profile_version(version: ListIntakeProfileVersionRequest) -> &'static str {
    match version {
        ListIntakeProfileVersionRequest::ListIntakeV2 => "\"list-intake-v2\"",
    }
}

const fn provider_prompt_version(version: ListIntakeProviderPromptVersionRequest) -> &'static str {
    match version {
        ListIntakeProviderPromptVersionRequest::ListIntakeV1 => "\"list-intake-v1\"",
    }
}

const fn output_policy(policy: ListIntakeOutputPolicyRequest) -> &'static str {
    match policy {
        ListIntakeOutputPolicyRequest::AggregateOnly => "\"aggregateOnly\"",
        ListIntakeOutputPolicyRequest::DailyDigestReminder => "\"dailyDigestReminder\"",
    }
}
