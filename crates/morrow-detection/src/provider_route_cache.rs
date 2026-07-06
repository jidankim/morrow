use std::error::Error;
use std::fmt::{Display, Formatter};

use morrow_messages::MessageEvidence;

use crate::types::{CivilDateTime, DetectionConfig};

pub trait ProviderRouteCache {
    type Error: Error + 'static;

    fn resolve_provider_route(
        &self,
        request: ProviderRouteRequest<'_>,
    ) -> Result<ProviderRouteDecision, Self::Error>;
}

#[derive(Debug, Clone, Copy)]
pub struct ProviderRouteRequest<'a> {
    pub message: &'a MessageEvidence,
    pub config: &'a DetectionConfig,
    pub parser_route_reason: &'static str,
    pub parser_time: Option<CivilDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderRouteDecision {
    Hit(CachedProviderOutcome),
    Miss {
        write_intent: Option<Box<ProviderRouteWriteIntent>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedProviderOutcome {
    pub route_fingerprint: String,
    pub outcome_kind: ProviderRouteOutcomeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderRouteOutcomeKind {
    Candidate,
    QuietLog,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRouteWriteIntent {
    pub route_fingerprint: String,
    pub provider_route_contract_version: String,
    pub provider_candidate_schema_version: String,
    pub evidence_payload_hash: String,
    pub provider_id: String,
    pub model_id: String,
    pub prompt_version: String,
    pub profile_id: String,
    pub profile_version: String,
    pub profile_schema_version: String,
    pub profile_policy_version: String,
    pub source_excerpt_policy: String,
    pub reference_observed: String,
    pub reference_timezone: String,
    pub threshold_millis: i64,
    pub parser_route: String,
}

#[derive(Debug, Clone, Copy)]
pub struct NoopProviderRouteCache;

impl ProviderRouteCache for NoopProviderRouteCache {
    type Error = NoopProviderRouteCacheError;

    fn resolve_provider_route(
        &self,
        _request: ProviderRouteRequest<'_>,
    ) -> Result<ProviderRouteDecision, Self::Error> {
        Ok(ProviderRouteDecision::Miss { write_intent: None })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoopProviderRouteCacheError {}

impl Display for NoopProviderRouteCacheError {
    fn fmt(&self, _formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {}
    }
}

impl Error for NoopProviderRouteCacheError {}
