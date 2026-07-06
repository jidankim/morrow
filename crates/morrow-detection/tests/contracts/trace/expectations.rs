use morrow_diagnostics::{TraceDecision, TraceOperation, TraceOutcome};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ExpectedTrace {
    pub(super) operation: TraceOperation,
    pub(super) decision: Option<TraceDecision>,
    pub(super) outcome: TraceOutcome,
    pub(super) reason_code: Option<&'static str>,
}

pub(super) fn provider_success_expectations() -> [ExpectedTrace; 7] {
    [
        parser_provider_route("parser_provider_route_ambiguous_calendar"),
        provider_route(),
        provider_success(),
        ExpectedTrace {
            operation: TraceOperation::SchemaValidation,
            decision: Some(TraceDecision::Candidate),
            outcome: TraceOutcome::Noop,
            reason_code: Some("provider_schema_accepted"),
        },
        ExpectedTrace {
            operation: TraceOperation::ThresholdDecision,
            decision: Some(TraceDecision::ConfidenceAccepted),
            outcome: TraceOutcome::CandidateCreated,
            reason_code: Some("confidence_meets_threshold"),
        },
        candidate_outcome(),
        privacy_hidden(),
    ]
}

pub(super) fn schema_rejection_expectations(reason: &'static str) -> [ExpectedTrace; 6] {
    [
        parser_provider_route("parser_provider_route_ambiguous_calendar"),
        provider_route(),
        provider_success(),
        ExpectedTrace {
            operation: TraceOperation::SchemaValidation,
            decision: Some(TraceDecision::SchemaRejected),
            outcome: TraceOutcome::Rejected,
            reason_code: Some(reason),
        },
        quiet_outcome(reason),
        privacy_hidden(),
    ]
}

pub(super) fn threshold_rejection_expectations() -> [ExpectedTrace; 7] {
    [
        parser_provider_route("parser_provider_route_ambiguous_calendar"),
        provider_route(),
        provider_success(),
        ExpectedTrace {
            operation: TraceOperation::SchemaValidation,
            decision: Some(TraceDecision::Candidate),
            outcome: TraceOutcome::Noop,
            reason_code: Some("provider_schema_accepted"),
        },
        ExpectedTrace {
            operation: TraceOperation::ThresholdDecision,
            decision: Some(TraceDecision::ConfidenceRejected),
            outcome: TraceOutcome::Rejected,
            reason_code: Some("confidence_below_threshold"),
        },
        quiet_outcome("confidence_below_threshold"),
        privacy_hidden(),
    ]
}

pub(super) fn parser_stop_expectations(reason: &'static str) -> [ExpectedTrace; 3] {
    [
        ExpectedTrace {
            operation: TraceOperation::ParserDecision,
            decision: Some(TraceDecision::Stop),
            outcome: TraceOutcome::Rejected,
            reason_code: Some(reason),
        },
        quiet_outcome(reason),
        privacy_hidden(),
    ]
}

pub(super) fn parser_candidate_expectations() -> [ExpectedTrace; 3] {
    [
        ExpectedTrace {
            operation: TraceOperation::ParserDecision,
            decision: Some(TraceDecision::Candidate),
            outcome: TraceOutcome::CandidateCreated,
            reason_code: Some("parser_candidate"),
        },
        candidate_outcome(),
        privacy_hidden(),
    ]
}

pub(super) fn provider_unavailable_expectations() -> [ExpectedTrace; 5] {
    [
        parser_provider_route("parser_provider_route_ambiguous_calendar"),
        provider_route(),
        ExpectedTrace {
            operation: TraceOperation::ProviderResult,
            decision: Some(TraceDecision::ProviderUnavailable),
            outcome: TraceOutcome::QuietLogged,
            reason_code: Some("provider_unavailable"),
        },
        quiet_outcome("provider_unavailable"),
        privacy_hidden(),
    ]
}

fn parser_provider_route(reason: &'static str) -> ExpectedTrace {
    ExpectedTrace {
        operation: TraceOperation::ParserDecision,
        decision: Some(TraceDecision::ProviderRoute),
        outcome: TraceOutcome::Noop,
        reason_code: Some(reason),
    }
}

fn provider_route() -> ExpectedTrace {
    ExpectedTrace {
        operation: TraceOperation::ProviderRoute,
        decision: Some(TraceDecision::ProviderRoute),
        outcome: TraceOutcome::Noop,
        reason_code: Some("provider_route"),
    }
}

fn provider_success() -> ExpectedTrace {
    ExpectedTrace {
        operation: TraceOperation::ProviderResult,
        decision: None,
        outcome: TraceOutcome::Noop,
        reason_code: Some("provider_extract_success"),
    }
}

fn candidate_outcome() -> ExpectedTrace {
    ExpectedTrace {
        operation: TraceOperation::OutcomeMaterialized,
        decision: Some(TraceDecision::Candidate),
        outcome: TraceOutcome::CandidateCreated,
        reason_code: Some("candidate_materialized"),
    }
}

fn quiet_outcome(reason: &'static str) -> ExpectedTrace {
    ExpectedTrace {
        operation: TraceOperation::OutcomeMaterialized,
        decision: Some(TraceDecision::Stop),
        outcome: TraceOutcome::QuietLogged,
        reason_code: Some(reason),
    }
}

fn privacy_hidden() -> ExpectedTrace {
    ExpectedTrace {
        operation: TraceOperation::OutcomeMaterialized,
        decision: None,
        outcome: TraceOutcome::Noop,
        reason_code: Some("source_excerpt_hidden"),
    }
}
