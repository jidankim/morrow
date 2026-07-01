use std::cell::{Cell, RefCell};
use std::error::Error;
use std::fmt::{Display, Formatter};

use morrow_detection::{
    AiProvider, CachedProviderOutcome, ProviderError, ProviderRequest, ProviderResponse,
    ProviderRouteCache, ProviderRouteDecision, ProviderRouteOutcomeKind, ProviderRouteRequest,
    ProviderRouteWriteIntent,
};

use crate::support::CollectingRecorder;

#[derive(Clone)]
enum StaticDecision {
    Hit(CachedProviderOutcome),
    Miss(Option<Box<ProviderRouteWriteIntent>>),
    Error(CacheFailure),
}

pub(super) struct StaticProviderRouteCache {
    decision: RefCell<StaticDecision>,
    calls: Cell<usize>,
}

impl StaticProviderRouteCache {
    pub(super) fn hit(route_fingerprint: &str, outcome_kind: ProviderRouteOutcomeKind) -> Self {
        Self {
            decision: RefCell::new(StaticDecision::Hit(CachedProviderOutcome {
                route_fingerprint: route_fingerprint.to_owned(),
                outcome_kind,
            })),
            calls: Cell::new(0),
        }
    }

    pub(super) fn miss(write_intent: Option<ProviderRouteWriteIntent>) -> Self {
        Self {
            decision: RefCell::new(StaticDecision::Miss(write_intent.map(Box::new))),
            calls: Cell::new(0),
        }
    }

    pub(super) fn error(error: CacheFailure) -> Self {
        Self {
            decision: RefCell::new(StaticDecision::Error(error)),
            calls: Cell::new(0),
        }
    }

    pub(super) const fn calls(&self) -> usize {
        self.calls.get()
    }
}

impl ProviderRouteCache for StaticProviderRouteCache {
    type Error = CacheFailure;

    fn resolve_provider_route(
        &self,
        request: ProviderRouteRequest<'_>,
    ) -> Result<ProviderRouteDecision, Self::Error> {
        self.calls.set(self.calls.get() + 1);
        assert_eq!(request.config.provider.provider_id, "fake-provider");
        assert!(request.message.excerpt.contains("Friday afternoon"));
        match self.decision.borrow().clone() {
            StaticDecision::Hit(outcome) => Ok(ProviderRouteDecision::Hit(outcome)),
            StaticDecision::Miss(write_intent) => Ok(ProviderRouteDecision::Miss { write_intent }),
            StaticDecision::Error(error) => Err(error),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CacheFailure;

impl Display for CacheFailure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "cache failure")
    }
}

impl Error for CacheFailure {}

#[derive(Default)]
pub(super) struct UnavailableProvider {
    calls: Cell<usize>,
}

impl UnavailableProvider {
    pub(super) const fn calls(&self) -> usize {
        self.calls.get()
    }
}

impl AiProvider for UnavailableProvider {
    fn extract(&self, _request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        self.calls.set(self.calls.get() + 1);
        Err(ProviderError::Unavailable {
            reason: "network offline".to_owned(),
        })
    }
}

pub(super) fn write_intent(
    route_fingerprint: &str,
    evidence_payload_hash: &str,
    parser_route: &str,
) -> ProviderRouteWriteIntent {
    ProviderRouteWriteIntent {
        route_fingerprint: route_fingerprint.to_owned(),
        provider_route_contract_version: "provider-route-ledger-v1".to_owned(),
        provider_candidate_schema_version: "provider-candidate-schema-v1".to_owned(),
        evidence_payload_hash: evidence_payload_hash.to_owned(),
        provider_id: "fake-provider".to_owned(),
        model_id: "offline-contract".to_owned(),
        prompt_version: "prompt-v1".to_owned(),
        source_excerpt_policy: "include".to_owned(),
        reference_observed: "2026-06-25T09:00:00[Asia/Seoul]".to_owned(),
        reference_timezone: "Asia/Seoul".to_owned(),
        threshold_millis: 550,
        parser_route: parser_route.to_owned(),
    }
}

pub(super) fn trace_reasons(recorder: &CollectingRecorder) -> Result<Vec<String>, Box<dyn Error>> {
    Ok(recorder
        .records()?
        .iter()
        .filter_map(|record| record.span.reason_code.clone())
        .collect())
}
