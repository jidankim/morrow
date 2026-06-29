#![forbid(unsafe_code)]

mod parser;
mod pipeline;
mod provider;
mod schema;
mod trace;
mod trace_event;
mod types;

pub use pipeline::{DetectionOutcome, DetectionPipeline, DetectionReport};
pub use provider::{AiProvider, ProviderError, ProviderRequest, ProviderResponse};
pub use types::{
    ConfidenceThreshold, DetectionConfig, DetectionError, ProviderIdentity, ReferenceTime,
    SourceExcerptPolicy,
};
