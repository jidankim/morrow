use morrow_diagnostics::{NoopTraceRecorder, TraceRecorder};
use morrow_messages::MessageEvidence;

use crate::outcome::{
    candidate_from_parsed, candidate_from_provider, quiet, quiet_with_provider_diagnostic,
    DetectionOutcome, DetectionReport,
};
use crate::parser::{classify, GateDecision, ParsedCandidate};
use crate::provider::{AiProvider, ProviderError, ProviderRequest};
use crate::provider_route_cache::{
    NoopProviderRouteCache, NoopProviderRouteCacheError, ProviderRouteCache, ProviderRouteDecision,
    ProviderRouteRequest as CacheRequest, ProviderRouteWriteIntent,
};
use crate::schema::parse_provider_candidate;
use crate::trace::MessageTrace;
use crate::types::{CivilDateTime, DetectionConfig};

#[derive(Debug, thiserror::Error)]
pub enum DetectionPipelineError<E: std::error::Error + 'static> {
    #[error("provider route cache failed: {0}")]
    ProviderRouteCache(E),
}

#[derive(Debug, Clone, Copy)]
pub struct DetectionPipeline<'a, P> {
    provider: &'a P,
}

impl<'a, P: AiProvider> DetectionPipeline<'a, P> {
    pub const fn new(provider: &'a P) -> Self {
        Self { provider }
    }

    pub fn detect(
        &self,
        messages: &[MessageEvidence],
        config: &DetectionConfig,
    ) -> DetectionReport {
        let recorder = NoopTraceRecorder;
        self.detect_with_trace(messages, config, &recorder)
    }

    pub fn detect_with_trace<R: TraceRecorder + ?Sized>(
        &self,
        messages: &[MessageEvidence],
        config: &DetectionConfig,
        recorder: &R,
    ) -> DetectionReport {
        let cache = NoopProviderRouteCache;
        match self.detect_with_trace_and_provider_cache(messages, config, recorder, &cache) {
            Ok(report) => report,
            Err(DetectionPipelineError::ProviderRouteCache(error)) => match error {},
        }
    }

    pub fn detect_with_trace_and_provider_cache<R, C>(
        &self,
        messages: &[MessageEvidence],
        config: &DetectionConfig,
        recorder: &R,
        cache: &C,
    ) -> Result<DetectionReport, DetectionPipelineError<C::Error>>
    where
        R: TraceRecorder + ?Sized,
        C: ProviderRouteCache + ?Sized,
    {
        let mut report = DetectionReport::default();
        for message in messages {
            let trace = MessageTrace::new(message, recorder);
            let step = self.detect_one(message, config, &trace, cache)?;
            report.outcomes.push(step.outcome);
            report
                .provider_route_write_intents
                .push(step.provider_route_write_intent);
        }
        Ok(report)
    }

    fn detect_one<R, C>(
        &self,
        message: &MessageEvidence,
        config: &DetectionConfig,
        trace: &MessageTrace<'_, R>,
        cache: &C,
    ) -> Result<DetectionStep, DetectionPipelineError<C::Error>>
    where
        R: TraceRecorder + ?Sized,
        C: ProviderRouteCache + ?Sized,
    {
        match classify(message, config) {
            GateDecision::Stop { reason } => {
                trace.parser_stop(reason, config);
                let outcome = quiet(message, reason, config.source_excerpts);
                trace.outcome_materialized(&outcome, config.source_excerpts);
                Ok(DetectionStep::new(outcome, None))
            }
            GateDecision::Candidate(parsed) => {
                trace.parser_candidate(parsed.confidence_millis, config);
                let outcome = candidate_from_parsed(message, parsed, &message.excerpt, config);
                trace.outcome_materialized(&outcome, config.source_excerpts);
                Ok(DetectionStep::new(outcome, None))
            }
            GateDecision::ProviderRoute {
                parser_time,
                fallback,
            } => {
                trace.parser_provider_route(config);
                self.detect_with_provider_cache(
                    message,
                    parser_time,
                    fallback,
                    config,
                    trace,
                    cache,
                )
            }
        }
    }

    fn detect_with_provider_cache<R, C>(
        &self,
        message: &MessageEvidence,
        parser_time: Option<CivilDateTime>,
        fallback: Option<ParsedCandidate>,
        config: &DetectionConfig,
        trace: &MessageTrace<'_, R>,
        cache: &C,
    ) -> Result<DetectionStep, DetectionPipelineError<C::Error>>
    where
        R: TraceRecorder + ?Sized,
        C: ProviderRouteCache + ?Sized,
    {
        let request = CacheRequest {
            message,
            config,
            parser_time,
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
                message,
                parser_time,
                fallback,
                config,
                trace,
                write_intent.map(|intent| *intent),
            )),
        }
    }

    fn detect_with_provider<R: TraceRecorder + ?Sized>(
        &self,
        message: &MessageEvidence,
        parser_time: Option<CivilDateTime>,
        fallback: Option<ParsedCandidate>,
        config: &DetectionConfig,
        trace: &MessageTrace<'_, R>,
        write_intent: Option<ProviderRouteWriteIntent>,
    ) -> DetectionStep {
        let evidence = std::slice::from_ref(message);
        trace.provider_route(config);
        let response = match self.provider.extract(ProviderRequest::new(
            evidence,
            &config.provider,
            &config.reference.timezone,
        )) {
            Ok(response) => {
                trace.provider_extract_success(config);
                response
            }
            Err(ProviderError::Unavailable { reason }) => {
                if let Some(parsed) = fallback {
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
        let outcome =
            match parse_provider_candidate(response.raw_json(), evidence, parser_time, config) {
                Ok(provider_candidate)
                    if provider_candidate.parsed.confidence_millis >= config.threshold.as_i64() =>
                {
                    trace.provider_schema_accepted(
                        provider_candidate.parsed.confidence_millis,
                        config,
                    );
                    trace.threshold_accepted(provider_candidate.parsed.confidence_millis, config);
                    let outcome = candidate_from_provider(
                        message,
                        provider_candidate,
                        config.source_excerpts,
                    );
                    trace.outcome_materialized(&outcome, config.source_excerpts);
                    outcome
                }
                Ok(provider_candidate) => {
                    trace.provider_schema_accepted(
                        provider_candidate.parsed.confidence_millis,
                        config,
                    );
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

struct DetectionStep {
    outcome: DetectionOutcome,
    provider_route_write_intent: Option<ProviderRouteWriteIntent>,
}

impl DetectionStep {
    const fn new(
        outcome: DetectionOutcome,
        provider_route_write_intent: Option<ProviderRouteWriteIntent>,
    ) -> Self {
        Self {
            outcome,
            provider_route_write_intent,
        }
    }
}

const _: fn(NoopProviderRouteCacheError) -> DetectionPipelineError<NoopProviderRouteCacheError> =
    DetectionPipelineError::ProviderRouteCache;
