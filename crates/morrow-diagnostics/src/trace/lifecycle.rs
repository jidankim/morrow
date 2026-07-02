use super::{
    TraceComponent, TraceDecision, TraceIds, TraceOperation, TraceOutcome, TracePrivacyTier,
    TraceRecord, TraceSchemaVersion, TraceSpan,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LifecycleReplayTraceTemplate {
    sequence: u8,
    component: TraceComponent,
    operation: TraceOperation,
    decision: TraceDecision,
    outcome: TraceOutcome,
    reason_code: &'static str,
}

pub(super) fn sample_lifecycle_replay_v1(operation: TraceOperation) -> Option<TraceRecord> {
    let template = lifecycle_replay_trace_template(operation)?;
    Some(TraceRecord {
        schema_version: TraceSchemaVersion::V1,
        trace: sample_lifecycle_replay_ids(template.sequence),
        span: sample_lifecycle_replay_span(template),
    })
}

fn sample_lifecycle_replay_ids(sequence: u8) -> TraceIds {
    TraceIds {
        trace_id: format!("trace_{:032x}", sequence),
        span_id: format!("span_{:032x}", sequence),
        parent_span_id: None,
        chat_hash: None,
        message_hash: None,
    }
}

fn sample_lifecycle_replay_span(template: LifecycleReplayTraceTemplate) -> TraceSpan {
    TraceSpan {
        component: template.component,
        operation: template.operation,
        decision: Some(template.decision),
        outcome: template.outcome,
        started_at: format!("2026-06-28T05:00:0{}Z", template.sequence),
        ended_at: Some(format!("2026-06-28T05:00:0{}Z", template.sequence)),
        provider_id: None,
        model_id: None,
        template_version: None,
        reason_code: Some(template.reason_code.to_owned()),
        confidence_millis: None,
        title_hash: None,
        title_status: Some("absent".to_owned()),
        privacy_tier: TracePrivacyTier::InternalMetadata,
        classifier_stage: None,
        router_stage: None,
        ood_score_millis: None,
        replay_run_id: Some("phase3-replay-local-001".to_owned()),
    }
}

fn lifecycle_replay_trace_template(
    operation: TraceOperation,
) -> Option<LifecycleReplayTraceTemplate> {
    match operation {
        TraceOperation::CandidateSuperseded => Some(LifecycleReplayTraceTemplate {
            sequence: 1,
            component: TraceComponent::Lifecycle,
            operation,
            decision: TraceDecision::LifecycleUpdated,
            outcome: TraceOutcome::Superseded,
            reason_code: "candidate_superseded",
        }),
        TraceOperation::CandidateRescheduled => Some(LifecycleReplayTraceTemplate {
            sequence: 2,
            component: TraceComponent::Lifecycle,
            operation,
            decision: TraceDecision::LifecycleUpdated,
            outcome: TraceOutcome::Rescheduled,
            reason_code: "candidate_rescheduled",
        }),
        TraceOperation::CandidateCancelled => Some(LifecycleReplayTraceTemplate {
            sequence: 3,
            component: TraceComponent::Lifecycle,
            operation,
            decision: TraceDecision::LifecycleUpdated,
            outcome: TraceOutcome::Cancelled,
            reason_code: "candidate_cancelled",
        }),
        TraceOperation::CalendarDryRun => Some(LifecycleReplayTraceTemplate {
            sequence: 4,
            component: TraceComponent::Calendar,
            operation,
            decision: TraceDecision::LifecycleUpdated,
            outcome: TraceOutcome::DryRun,
            reason_code: "calendar_dry_run",
        }),
        TraceOperation::CalendarCommitIdempotency => Some(LifecycleReplayTraceTemplate {
            sequence: 5,
            component: TraceComponent::Calendar,
            operation,
            decision: TraceDecision::LifecycleUpdated,
            outcome: TraceOutcome::CommitIdempotent,
            reason_code: "calendar_commit_idempotency",
        }),
        TraceOperation::ReplayRun => Some(LifecycleReplayTraceTemplate {
            sequence: 6,
            component: TraceComponent::Replay,
            operation,
            decision: TraceDecision::ReplayCompared,
            outcome: TraceOutcome::ReplayRecorded,
            reason_code: "replay_run",
        }),
        TraceOperation::ParserDecision
        | TraceOperation::ProviderRoute
        | TraceOperation::ProviderResult
        | TraceOperation::SchemaValidation
        | TraceOperation::ThresholdDecision
        | TraceOperation::OutcomeMaterialized
        | TraceOperation::ClassifierScore
        | TraceOperation::RouterDecision
        | TraceOperation::OodCheck
        | TraceOperation::PollEmpty
        | TraceOperation::CursorAdvanced
        | TraceOperation::UserCorrection => None,
    }
}
