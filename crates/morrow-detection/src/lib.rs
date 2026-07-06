#![forbid(unsafe_code)]

mod outcome;
mod parser;
mod pipeline;
mod provider;
mod provider_route_cache;
mod schema;
mod trace;
mod trace_event;
mod types;

pub use outcome::{DetectionOutcome, DetectionReport};
pub use pipeline::{DetectionPipeline, DetectionPipelineError};
pub use provider::{AiProvider, ProviderError, ProviderRequest, ProviderResponse};
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
