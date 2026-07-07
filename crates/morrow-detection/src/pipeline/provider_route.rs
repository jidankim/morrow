use morrow_diagnostics::TraceRecorder;
use morrow_messages::MessageEvidence;

use super::{DetectionPipeline, DetectionPipelineError, DetectionStep, ProviderRoutePlan};
use crate::outcome::{
    candidate_from_parsed, candidate_from_provider, quiet, quiet_with_provider_diagnostic,
    DetectionOutcome,
};
use crate::provider::{AiProvider, ProviderError, ProviderRequest};
use crate::provider_route_cache::{
    ProviderRouteCache, ProviderRouteDecision, ProviderRouteRequest as CacheRequest,
    ProviderRouteWriteIntent,
};
use crate::schema::parse_provider_candidate;
use crate::trace::MessageTrace;
use crate::types::DetectionConfig;

const PROVIDER_ANCHOR_NOT_CURRENT_MESSAGE: &str = "provider_anchor_not_current_message";
const PROVIDER_UNSUPPORTED_LIFECYCLE: &str = "provider_unsupported_lifecycle";
const PROVIDER_ROUTE_LIFECYCLE_UPDATE: &str = "parser_provider_route_lifecycle_update";

impl<'a, P: AiProvider> DetectionPipeline<'a, P> {
    pub(super) fn detect_with_provider_cache<R, C>(
        &self,
        selected_evidence: &[MessageEvidence],
        message: &MessageEvidence,
        route: ProviderRoutePlan,
        config: &DetectionConfig,
        trace: &MessageTrace<'_, R>,
        cache: &C,
    ) -> Result<DetectionStep, DetectionPipelineError<C::Error>>
    where
        R: TraceRecorder + ?Sized,
        C: ProviderRouteCache + ?Sized,
    {
        let request = CacheRequest {
            selected_evidence,
            message,
            config,
            parser_route_reason: route.reason,
            parser_time: route.parser_time,
        };
        match cache
            .resolve_provider_route(request)
            .map_err(DetectionPipelineError::ProviderRouteCache)?
        {
            ProviderRouteDecision::Hit(cached) => {
                trace.provider_route_cache_hit(config);
                let outcome = DetectionOutcome::CachedProviderRoute {
                    route_fingerprint: cached.route_fingerprint,
                    outcome_kind: cached.outcome_kind,
                };
                trace.outcome_materialized(&outcome, config.source_excerpts);
                Ok(DetectionStep::new(outcome, None))
            }
            ProviderRouteDecision::Miss { write_intent } => Ok(self.detect_with_provider(
                selected_evidence,
                message,
                route,
                config,
                trace,
                write_intent.map(|intent| *intent),
            )),
        }
    }

    fn detect_with_provider<R: TraceRecorder + ?Sized>(
        &self,
        selected_evidence: &[MessageEvidence],
        message: &MessageEvidence,
        route: ProviderRoutePlan,
        config: &DetectionConfig,
        trace: &MessageTrace<'_, R>,
        write_intent: Option<ProviderRouteWriteIntent>,
    ) -> DetectionStep {
        trace.provider_route(config);
        let response = match self.provider.extract(ProviderRequest::new(
            selected_evidence,
            &config.provider,
            &config.reference.timezone,
        )) {
            Ok(response) => {
                trace.provider_extract_success(config);
                response
            }
            Err(ProviderError::Unavailable { reason }) => {
                if let Some(parsed) = route.fallback {
                    trace.provider_unavailable(Some(parsed.confidence_millis), config);
                    let outcome = candidate_from_parsed(message, parsed, &message.excerpt, config);
                    trace.outcome_materialized(&outcome, config.source_excerpts);
                    return DetectionStep::new(outcome, None);
                }
                trace.provider_unavailable(None, config);
                let outcome = quiet_with_provider_diagnostic(
                    message,
                    "provider_unavailable",
                    reason,
                    config.source_excerpts,
                );
                trace.outcome_materialized(&outcome, config.source_excerpts);
                return DetectionStep::new(outcome, None);
            }
        };
        let outcome = match parse_provider_candidate(
            response.raw_json(),
            selected_evidence,
            route.parser_time,
            config,
        ) {
            Ok(provider_candidate)
                if provider_candidate.anchor_message_guid != message.message_guid.as_str() =>
            {
                trace.provider_schema_rejected(PROVIDER_ANCHOR_NOT_CURRENT_MESSAGE, config);
                let outcome = quiet(
                    message,
                    PROVIDER_ANCHOR_NOT_CURRENT_MESSAGE,
                    config.source_excerpts,
                );
                trace.outcome_materialized(&outcome, config.source_excerpts);
                outcome
            }
            Ok(_) if route.reason == PROVIDER_ROUTE_LIFECYCLE_UPDATE => {
                trace.provider_schema_rejected(PROVIDER_UNSUPPORTED_LIFECYCLE, config);
                let outcome = quiet(
                    message,
                    PROVIDER_UNSUPPORTED_LIFECYCLE,
                    config.source_excerpts,
                );
                trace.outcome_materialized(&outcome, config.source_excerpts);
                outcome
            }
            Ok(provider_candidate)
                if provider_candidate.parsed.confidence_millis >= config.threshold.as_i64() =>
            {
                trace.provider_schema_accepted(provider_candidate.parsed.confidence_millis, config);
                trace.threshold_accepted(provider_candidate.parsed.confidence_millis, config);
                let outcome =
                    candidate_from_provider(message, provider_candidate, config.source_excerpts);
                trace.outcome_materialized(&outcome, config.source_excerpts);
                outcome
            }
            Ok(provider_candidate) => {
                trace.provider_schema_accepted(provider_candidate.parsed.confidence_millis, config);
                trace.threshold_rejected(provider_candidate.parsed.confidence_millis, config);
                let outcome = quiet(
                    message,
                    "confidence_below_threshold",
                    config.source_excerpts,
                );
                trace.outcome_materialized(&outcome, config.source_excerpts);
                outcome
            }
            Err(rejection) => {
                let reason = rejection.reason();
                trace.provider_schema_rejected(reason, config);
                let outcome = quiet(message, reason, config.source_excerpts);
                trace.outcome_materialized(&outcome, config.source_excerpts);
                outcome
            }
        };
        DetectionStep::new(outcome, write_intent)
    }
}
