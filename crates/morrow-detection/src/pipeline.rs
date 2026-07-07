use morrow_diagnostics::{NoopTraceRecorder, TraceRecorder};
use morrow_messages::MessageEvidence;

mod provider_route;

use crate::outcome::{candidate_from_parsed, quiet, DetectionOutcome, DetectionReport};
use crate::parser::{classify, GateDecision, ParsedCandidate};
use crate::provider::AiProvider;
use crate::provider_route_cache::{
    NoopProviderRouteCache, NoopProviderRouteCacheError, ProviderRouteCache,
    ProviderRouteWriteIntent,
};
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
            let step = self.detect_one(messages, message, config, &trace, cache)?;
            report.outcomes.push(step.outcome);
            report
                .provider_route_write_intents
                .push(step.provider_route_write_intent);
        }
        Ok(report)
    }

    fn detect_one<R, C>(
        &self,
        selected_evidence: &[MessageEvidence],
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
                reason,
                parser_time,
                fallback,
            } => {
                trace.parser_provider_route(reason, config);
                let route = ProviderRoutePlan {
                    reason,
                    parser_time,
                    fallback,
                };
                self.detect_with_provider_cache(
                    selected_evidence,
                    message,
                    route,
                    config,
                    trace,
                    cache,
                )
            }
        }
    }
}

pub(super) struct DetectionStep {
    pub(super) outcome: DetectionOutcome,
    pub(super) provider_route_write_intent: Option<ProviderRouteWriteIntent>,
}

pub(super) struct ProviderRoutePlan {
    pub(super) reason: &'static str,
    pub(super) parser_time: Option<CivilDateTime>,
    pub(super) fallback: Option<ParsedCandidate>,
}

impl DetectionStep {
    pub(super) const fn new(
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
