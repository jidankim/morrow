#![forbid(unsafe_code)]

mod list_intake;
mod outcome;
mod parser;
mod pipeline;
mod provider;
mod provider_route_cache;
mod schema;
mod trace;
mod trace_event;
mod types;

pub use list_intake::{
    plan_list_intake_extraction, validate_list_intake_provider_output, ListIntakeCategoryRule,
    ListIntakeChatScope, ListIntakeConfidenceTier, ListIntakeLocalDayWindow, ListIntakeProfile,
    ListIntakeProfileKind, ListIntakeProviderExtractionRequest, ListIntakeRouteDecision,
    ListIntakeSchedulingDecision, ListIntakeValidationError, ValidatedListIntakeExtraction,
    ValidatedListIntakeItem, LIST_INTAKE_PROVIDER_SCHEMA_VERSION,
    LIST_INTAKE_UNCATEGORIZED_CATEGORY_ID,
};
pub use outcome::{DetectionOutcome, DetectionReport};
pub use pipeline::{DetectionPipeline, DetectionPipelineError};
pub use provider::{
    AiProvider, ListIntakeProviderRequest, ProviderError, ProviderRequest, ProviderResponse,
};
pub use provider_route_cache::{
    CachedProviderOutcome, ProviderRouteCache, ProviderRouteDecision, ProviderRouteOutcomeKind,
    ProviderRouteRequest, ProviderRouteWriteIntent,
};
pub use types::{
    CivilDateTime, ConfidenceThreshold, DetectionConfig, DetectionError,
    ListReminderDefaultDueMode, ListReminderDefaultDueTime, ListReminderItemOutputMode,
    ListReminderProfile, ListReminderProfileId, ListReminderProfileVersion,
    ListReminderRecurrenceMode, ListReminderRoutingMode, ProviderIdentity, ReferenceTime,
    SourceExcerptPolicy,
};
