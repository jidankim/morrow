use morrow_diagnostics::{TraceComponent, TraceDecision, TraceOperation, TraceOutcome};

pub(super) const fn trace_component_name(component: TraceComponent) -> &'static str {
    match component {
        TraceComponent::Detection => "detection",
        TraceComponent::Parser => "parser",
        TraceComponent::Provider => "provider",
        TraceComponent::Schema => "schema",
        TraceComponent::Threshold => "threshold",
        TraceComponent::Outcome => "outcome",
        TraceComponent::Storage => "storage",
        TraceComponent::Classifier => "classifier",
        TraceComponent::Router => "router",
        TraceComponent::Ood => "ood",
        TraceComponent::Lifecycle => "lifecycle",
        TraceComponent::Correction => "correction",
        TraceComponent::Calendar => "calendar",
        TraceComponent::Replay => "replay",
    }
}

pub(super) const fn trace_operation_name(operation: TraceOperation) -> &'static str {
    match operation {
        TraceOperation::ParserDecision => "parser_decision",
        TraceOperation::ProviderRoute => "provider_route",
        TraceOperation::ProviderResult => "provider_result",
        TraceOperation::SchemaValidation => "schema_validation",
        TraceOperation::ThresholdDecision => "threshold_decision",
        TraceOperation::OutcomeMaterialized => "outcome_materialized",
        TraceOperation::ClassifierScore => "classifier_score",
        TraceOperation::RouterDecision => "router_decision",
        TraceOperation::OodCheck => "ood_check",
        TraceOperation::PollEmpty => "poll_empty",
        TraceOperation::CursorAdvanced => "cursor_advanced",
        TraceOperation::UserCorrection => "user_correction",
        TraceOperation::CandidateSuperseded => "candidate_superseded",
        TraceOperation::CandidateRescheduled => "candidate_rescheduled",
        TraceOperation::CandidateCancelled => "candidate_cancelled",
        TraceOperation::CalendarDryRun => "calendar_dry_run",
        TraceOperation::CalendarCommitIdempotency => "calendar_commit_idempotency",
        TraceOperation::ReplayRun => "replay_run",
    }
}

pub(super) const fn trace_decision_name(decision: TraceDecision) -> &'static str {
    match decision {
        TraceDecision::Stop => "stop",
        TraceDecision::Candidate => "candidate",
        TraceDecision::ProviderRoute => "provider_route",
        TraceDecision::ProviderUnavailable => "provider_unavailable",
        TraceDecision::SchemaRejected => "schema_rejected",
        TraceDecision::ConfidenceAccepted => "confidence_accepted",
        TraceDecision::ConfidenceRejected => "confidence_rejected",
        TraceDecision::ClassifierAccepted => "classifier_accepted",
        TraceDecision::ClassifierRejected => "classifier_rejected",
        TraceDecision::RouterAccepted => "router_accepted",
        TraceDecision::RouterFallback => "router_fallback",
        TraceDecision::OodAccepted => "ood_accepted",
        TraceDecision::OodRejected => "ood_rejected",
        TraceDecision::UserCorrected => "user_corrected",
        TraceDecision::LifecycleUpdated => "lifecycle_updated",
        TraceDecision::ReplayCompared => "replay_compared",
    }
}

pub(super) const fn trace_outcome_name(outcome: TraceOutcome) -> &'static str {
    match outcome {
        TraceOutcome::CandidateCreated => "candidate_created",
        TraceOutcome::QuietLogged => "quiet_logged",
        TraceOutcome::Noop => "noop",
        TraceOutcome::Rejected => "rejected",
        TraceOutcome::Superseded => "superseded",
        TraceOutcome::Rescheduled => "rescheduled",
        TraceOutcome::Cancelled => "cancelled",
        TraceOutcome::DryRun => "dry_run",
        TraceOutcome::CommitIdempotent => "commit_idempotent",
        TraceOutcome::ReplayRecorded => "replay_recorded",
    }
}
