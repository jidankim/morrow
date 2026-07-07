#[path = "list_intake_examples_hash.rs"]
mod examples_hash;
#[path = "list_intake_validation.rs"]
mod validation;

pub(super) use validation::deserialize_profiles;

use morrow_detection::{
    ListIntakeCategoryRule, ListIntakeChatScope, ListIntakeProfile, ListIntakeProfileKind,
};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ListIntakeProfileRequest {
    pub enabled: bool,
    pub profile_id: String,
    pub name: String,
    pub profile_version: ListIntakeProfileVersionRequest,
    pub kind: ListIntakeProfileKindRequest,
    pub extraction_mode: ListIntakeExtractionModeRequest,
    pub provider_prompt_version: ListIntakeProviderPromptVersionRequest,
    pub positive_examples: Vec<String>,
    pub negative_examples: Vec<String>,
    pub category_rules: Vec<ListIntakeCategoryRuleRequest>,
    pub examples_hash: String,
    pub aggregation: ListIntakeAggregationRequest,
    pub chat_scope: ListIntakeChatScopeRequest,
    pub grouping: ListIntakeGroupingRequest,
    pub capture_from_scheduled_messages: bool,
    pub output_policy: ListIntakeOutputPolicyRequest,
    pub digest_reminder: Option<ListIntakeDigestReminderRequest>,
    pub quantity_list_bounds: ListIntakeQuantityListBoundsRequest,
    pub thresholds: ListIntakeThresholdsRequest,
    pub migration_state: Option<ListIntakeMigrationStateRequest>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum ListIntakeProfileVersionRequest {
    #[serde(rename = "list-intake-v2")]
    ListIntakeV2,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum ListIntakeProfileKindRequest {
    #[serde(rename = "quantityList")]
    QuantityList,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum ListIntakeExtractionModeRequest {
    #[serde(rename = "providerConstrained")]
    ProviderConstrained,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum ListIntakeProviderPromptVersionRequest {
    #[serde(rename = "list-intake-v1")]
    ListIntakeV1,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ListIntakeCategoryRuleRequest {
    pub category_id: String,
    pub display_name: String,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ListIntakeAggregationRequest {
    pub window: ListIntakeAggregationWindowRequest,
    pub timezone_source: ListIntakeTimezoneSourceRequest,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum ListIntakeAggregationWindowRequest {
    #[serde(rename = "localDay")]
    LocalDay,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum ListIntakeTimezoneSourceRequest {
    #[serde(rename = "referenceTimezone")]
    ReferenceTimezone,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub enum ListIntakeChatScopeRequest {
    AllSelectedChats {},
    SelectedChatIds {
        #[serde(rename = "selectedChatIds")]
        selected_chat_ids: Vec<String>,
    },
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ListIntakeGroupingRequest {
    pub chat: bool,
    pub sender: ListIntakeSenderGroupingRequest,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ListIntakeSenderGroupingRequest {
    Off,
    DisplayAlias,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ListIntakeOutputPolicyRequest {
    AggregateOnly,
    DailyDigestReminder,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ListIntakeDigestReminderRequest {
    pub due_time_local: ListIntakeDigestDueTimeRequest,
    pub date_offset_days: u8,
    pub output_policy_version: ListIntakeDigestOutputPolicyVersionRequest,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum ListIntakeDigestDueTimeRequest {
    #[serde(rename = "09:00")]
    NineLocal,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum ListIntakeDigestOutputPolicyVersionRequest {
    #[serde(rename = "list-intake-digest-v1")]
    ListIntakeDigestV1,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ListIntakeQuantityListBoundsRequest {
    pub min_items: u8,
    pub max_items: u8,
    pub min_quantity: u16,
    pub max_quantity: u16,
    pub max_item_name_visible_chars: u8,
    pub max_unit_visible_chars: u8,
    pub uncategorized_category_id: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ListIntakeThresholdsRequest {
    pub auto_aggregate_threshold_millis: u16,
    pub review_threshold_millis: u16,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum ListIntakeMigrationStateRequest {
    #[serde(rename = "needsReviewFromListReminderV1")]
    NeedsReviewFromListReminderV1,
}

pub(super) fn detection_profile(profile: &ListIntakeProfileRequest) -> ListIntakeProfile {
    ListIntakeProfile {
        enabled: profile.enabled,
        profile_id: profile.profile_id.clone(),
        profile_version: profile_version(profile.profile_version).to_owned(),
        name: profile.name.clone(),
        kind: profile_kind(profile.kind),
        provider_prompt_version: provider_prompt_version(profile.provider_prompt_version)
            .to_owned(),
        positive_examples: profile.positive_examples.clone(),
        negative_examples: profile.negative_examples.clone(),
        category_rules: profile
            .category_rules
            .iter()
            .map(|rule| ListIntakeCategoryRule {
                category_id: rule.category_id.clone(),
                display_name: rule.display_name.clone(),
                keywords: rule.keywords.clone(),
            })
            .collect(),
        examples_hash: profile.examples_hash.clone(),
        chat_scope: chat_scope(&profile.chat_scope),
        capture_from_scheduled_messages: profile.capture_from_scheduled_messages,
    }
}

pub(super) fn rewrite_selected_chat_scope_ids(
    profiles: &mut [ListIntakeProfileRequest],
    public_ids: &[String],
    raw_ids: &[String],
) {
    for profile in profiles {
        let ListIntakeChatScopeRequest::SelectedChatIds { selected_chat_ids } =
            &mut profile.chat_scope
        else {
            continue;
        };
        for selected_chat_id in selected_chat_ids {
            if let Some(raw_id) = public_ids
                .iter()
                .position(|public_id| public_id == selected_chat_id)
                .and_then(|index| raw_ids.get(index))
            {
                *selected_chat_id = raw_id.clone();
            }
        }
    }
}

const fn profile_version(version: ListIntakeProfileVersionRequest) -> &'static str {
    match version {
        ListIntakeProfileVersionRequest::ListIntakeV2 => "list-intake-v2",
    }
}

const fn profile_kind(kind: ListIntakeProfileKindRequest) -> ListIntakeProfileKind {
    match kind {
        ListIntakeProfileKindRequest::QuantityList => ListIntakeProfileKind::QuantityList,
    }
}

const fn provider_prompt_version(version: ListIntakeProviderPromptVersionRequest) -> &'static str {
    match version {
        ListIntakeProviderPromptVersionRequest::ListIntakeV1 => "list-intake-v1",
    }
}

fn chat_scope(scope: &ListIntakeChatScopeRequest) -> ListIntakeChatScope {
    match scope {
        ListIntakeChatScopeRequest::AllSelectedChats {} => ListIntakeChatScope::AllSelectedChats,
        ListIntakeChatScopeRequest::SelectedChatIds { selected_chat_ids } => {
            ListIntakeChatScope::SelectedChatIds(selected_chat_ids.clone())
        }
    }
}
