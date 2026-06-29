use morrow_diagnostics::{
    TraceComponent, TraceDecision, TraceIds, TraceOperation, TraceOutcome, TracePrivacyTier,
    TraceRecord, TraceRecorder, TraceSchemaVersion,
};
use morrow_messages::MessageEvidence;

use crate::pipeline::DetectionOutcome;
use crate::trace_event::{bounded_confidence, candidate_outcome, TraceEvent};
use crate::types::{DetectionConfig, SourceExcerptPolicy};

pub(crate) struct MessageTrace<'a, R: TraceRecorder + ?Sized> {
    recorder: &'a R,
    root: Option<TraceIds>,
    message_guid: &'a str,
    observed_at: String,
}

impl<'a, R: TraceRecorder + ?Sized> MessageTrace<'a, R> {
    pub(crate) fn new(message: &'a MessageEvidence, recorder: &'a R) -> Self {
        Self {
            recorder,
            root: TraceIds::random(
                Some(message.chat_guid.as_str()),
                Some(message.message_guid.as_str()),
            )
            .ok(),
            message_guid: message.message_guid.as_str(),
            observed_at: format!("message_timestamp:{}", message.timestamp.as_i64()),
        }
    }

    pub(crate) fn parser_stop(&self, reason: &'static str, config: &DetectionConfig) {
        self.record_root(TraceEvent {
            component: TraceComponent::Parser,
            operation: TraceOperation::ParserDecision,
            decision: Some(TraceDecision::Stop),
            outcome: TraceOutcome::Rejected,
            reason_code: Some(reason),
            confidence_millis: None,
            title: None,
            privacy_tier: TracePrivacyTier::InternalMetadata,
            provider: Some(&config.provider),
        });
    }

    pub(crate) fn parser_candidate(&self, confidence_millis: i64, config: &DetectionConfig) {
        self.record_root(TraceEvent {
            component: TraceComponent::Parser,
            operation: TraceOperation::ParserDecision,
            decision: Some(TraceDecision::Candidate),
            outcome: TraceOutcome::CandidateCreated,
            reason_code: Some("parser_candidate"),
            confidence_millis: bounded_confidence(confidence_millis),
            title: None,
            privacy_tier: TracePrivacyTier::InternalMetadata,
            provider: Some(&config.provider),
        });
    }

    pub(crate) fn parser_provider_route(&self, config: &DetectionConfig) {
        self.record_root(TraceEvent {
            component: TraceComponent::Parser,
            operation: TraceOperation::ParserDecision,
            decision: Some(TraceDecision::ProviderRoute),
            outcome: TraceOutcome::Noop,
            reason_code: Some("parser_provider_route"),
            confidence_millis: None,
            title: None,
            privacy_tier: TracePrivacyTier::InternalMetadata,
            provider: Some(&config.provider),
        });
    }

    pub(crate) fn provider_route(&self, config: &DetectionConfig) {
        self.record_child(TraceEvent {
            component: TraceComponent::Provider,
            operation: TraceOperation::ProviderRoute,
            decision: Some(TraceDecision::ProviderRoute),
            outcome: TraceOutcome::Noop,
            reason_code: Some("provider_route"),
            confidence_millis: None,
            title: None,
            privacy_tier: TracePrivacyTier::InternalMetadata,
            provider: Some(&config.provider),
        });
    }

    pub(crate) fn provider_extract_success(&self, config: &DetectionConfig) {
        self.record_child(TraceEvent {
            component: TraceComponent::Provider,
            operation: TraceOperation::ProviderResult,
            decision: None,
            outcome: TraceOutcome::Noop,
            reason_code: Some("provider_extract_success"),
            confidence_millis: None,
            title: None,
            privacy_tier: TracePrivacyTier::InternalMetadata,
            provider: Some(&config.provider),
        });
    }

    pub(crate) fn provider_unavailable(&self, config: &DetectionConfig) {
        self.record_child(TraceEvent {
            component: TraceComponent::Provider,
            operation: TraceOperation::ProviderResult,
            decision: Some(TraceDecision::ProviderUnavailable),
            outcome: TraceOutcome::QuietLogged,
            reason_code: Some("provider_unavailable"),
            confidence_millis: None,
            title: None,
            privacy_tier: TracePrivacyTier::InternalMetadata,
            provider: Some(&config.provider),
        });
    }

    pub(crate) fn provider_schema_accepted(
        &self,
        confidence_millis: i64,
        config: &DetectionConfig,
    ) {
        self.record_child(TraceEvent {
            component: TraceComponent::Schema,
            operation: TraceOperation::SchemaValidation,
            decision: Some(TraceDecision::Candidate),
            outcome: TraceOutcome::Noop,
            reason_code: Some("provider_schema_accepted"),
            confidence_millis: bounded_confidence(confidence_millis),
            title: None,
            privacy_tier: TracePrivacyTier::InternalMetadata,
            provider: Some(&config.provider),
        });
    }

    pub(crate) fn provider_schema_rejected(&self, reason: &'static str, config: &DetectionConfig) {
        self.record_child(TraceEvent {
            component: TraceComponent::Schema,
            operation: TraceOperation::SchemaValidation,
            decision: Some(TraceDecision::SchemaRejected),
            outcome: TraceOutcome::Rejected,
            reason_code: Some(reason),
            confidence_millis: None,
            title: None,
            privacy_tier: TracePrivacyTier::InternalMetadata,
            provider: Some(&config.provider),
        });
    }

    pub(crate) fn threshold_accepted(&self, confidence_millis: i64, config: &DetectionConfig) {
        self.record_child(TraceEvent {
            component: TraceComponent::Threshold,
            operation: TraceOperation::ThresholdDecision,
            decision: Some(TraceDecision::ConfidenceAccepted),
            outcome: TraceOutcome::CandidateCreated,
            reason_code: Some("confidence_meets_threshold"),
            confidence_millis: bounded_confidence(confidence_millis),
            title: None,
            privacy_tier: TracePrivacyTier::InternalMetadata,
            provider: Some(&config.provider),
        });
    }

    pub(crate) fn threshold_rejected(&self, confidence_millis: i64, config: &DetectionConfig) {
        self.record_child(TraceEvent {
            component: TraceComponent::Threshold,
            operation: TraceOperation::ThresholdDecision,
            decision: Some(TraceDecision::ConfidenceRejected),
            outcome: TraceOutcome::Rejected,
            reason_code: Some("confidence_below_threshold"),
            confidence_millis: bounded_confidence(confidence_millis),
            title: None,
            privacy_tier: TracePrivacyTier::InternalMetadata,
            provider: Some(&config.provider),
        });
    }

    pub(crate) fn outcome_materialized(
        &self,
        outcome: &DetectionOutcome,
        source_excerpts: SourceExcerptPolicy,
    ) {
        let event = match outcome {
            DetectionOutcome::Candidate(candidate) => candidate_outcome(candidate),
            DetectionOutcome::QuietLog(quiet) => TraceEvent {
                component: TraceComponent::Outcome,
                operation: TraceOperation::OutcomeMaterialized,
                decision: Some(TraceDecision::Stop),
                outcome: TraceOutcome::QuietLogged,
                reason_code: Some(&quiet.reason),
                confidence_millis: None,
                title: None,
                privacy_tier: TracePrivacyTier::InternalMetadata,
                provider: None,
            },
        };
        self.record_child(event);
        self.record_privacy_state(source_excerpts);
    }

    fn record_privacy_state(&self, source_excerpts: SourceExcerptPolicy) {
        let reason_code = match source_excerpts {
            SourceExcerptPolicy::Include => "source_excerpt_included",
            SourceExcerptPolicy::Hide => "source_excerpt_hidden",
        };
        self.record_child(TraceEvent {
            component: TraceComponent::Outcome,
            operation: TraceOperation::OutcomeMaterialized,
            decision: None,
            outcome: TraceOutcome::Noop,
            reason_code: Some(reason_code),
            confidence_millis: None,
            title: None,
            privacy_tier: TracePrivacyTier::LocalPrivate,
            provider: None,
        });
    }

    fn record_root(&self, event: TraceEvent<'_>) {
        if let Some(ids) = &self.root {
            self.emit(ids.clone(), event);
        }
    }

    fn record_child(&self, event: TraceEvent<'_>) {
        if let Some(parent) = &self.root {
            if let Ok(ids) = TraceIds::child(parent, Some(self.message_guid)) {
                self.emit(ids, event);
            }
        }
    }

    fn emit(&self, ids: TraceIds, event: TraceEvent<'_>) {
        let record = TraceRecord {
            schema_version: TraceSchemaVersion::V1,
            trace: ids,
            span: event.span(&self.observed_at),
        };
        drop(self.recorder.record(&record));
    }
}
