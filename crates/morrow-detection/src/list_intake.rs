#[path = "list_intake_route.rs"]
mod route;
#[path = "list_intake_validation.rs"]
mod validation;

pub use route::plan_list_intake_extraction;
pub use validation::validate_list_intake_provider_output;

pub const LIST_INTAKE_PROVIDER_SCHEMA_VERSION: &str = "list-intake-provider-schema-v1";
pub const LIST_INTAKE_UNCATEGORIZED_CATEGORY_ID: &str = "uncategorized";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListIntakeProfile {
    pub enabled: bool,
    pub profile_id: String,
    pub profile_version: String,
    pub name: String,
    pub kind: ListIntakeProfileKind,
    pub provider_prompt_version: String,
    pub positive_examples: Vec<String>,
    pub negative_examples: Vec<String>,
    pub category_rules: Vec<ListIntakeCategoryRule>,
    pub examples_hash: String,
    pub chat_scope: ListIntakeChatScope,
    pub capture_from_scheduled_messages: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListIntakeProfileKind {
    QuantityList,
}

impl ListIntakeProfileKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::QuantityList => "quantityList",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListIntakeCategoryRule {
    pub category_id: String,
    pub display_name: String,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListIntakeChatScope {
    AllSelectedChats,
    SelectedChatIds(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListIntakeSchedulingDecision {
    NotSchedulingOwned,
    SchedulingOwned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListIntakeRouteDecision {
    SkipSchedulingOwned { reason: &'static str },
    SkipProfileScope { reason: &'static str },
    SkipLocalSafety { reason: &'static str },
    ProviderExtraction(ListIntakeProviderExtractionRequest),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListIntakeProviderExtractionRequest {
    pub profile_id: String,
    pub profile_version: String,
    pub examples_hash: String,
    pub evidence_pointer: String,
    pub message_guid: String,
    pub reference_timezone: String,
    pub provider_prompt_version: String,
    pub provider_schema_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedListIntakeExtraction {
    pub profile_id: String,
    pub profile_version: String,
    pub examples_hash: String,
    pub evidence_pointer: String,
    pub message_guid: String,
    pub items: Vec<ValidatedListIntakeItem>,
    pub window: ListIntakeLocalDayWindow,
    pub confidence_tier: ListIntakeConfidenceTier,
    pub confidence_millis: u16,
    pub provider_prompt_version: String,
    pub provider_schema_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedListIntakeItem {
    pub name: String,
    pub quantity: u16,
    pub unit: Option<String>,
    pub category_id: String,
    pub evidence_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListIntakeLocalDayWindow {
    pub window_local_date: String,
    pub window_timezone: String,
    pub window_start_unix_seconds: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListIntakeConfidenceTier {
    AutoAggregate,
    Review,
    Low,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ListIntakeValidationError {
    #[error("provider output was not valid list-intake json")]
    MalformedJson,
    #[error("provider output failed list-intake contract: {reason}")]
    InvalidProviderOutput { reason: &'static str },
}
