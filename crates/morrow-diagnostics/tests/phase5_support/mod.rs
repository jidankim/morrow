use morrow_diagnostics::{
    TraceComponent, TraceDecision, TraceIds, TraceOperation, TraceOutcome, TracePrivacyTier,
    TraceRecord, TraceSchemaVersion, TraceSpan, TrajectoryAnchorReport, TrajectoryAnchorStep,
    TrajectoryAnchorStepInput,
};

pub const CASE_ID: &str = "phase5-case-calendar-accepted";
pub const FEATURE_SNAPSHOT_ID: &str = "phase5-case-calendar-accepted-snapshot";
pub const DECISION_EVIDENCE_SUBJECT_ID: &str = "candidate_trajectory_calendar_accept";

pub fn trajectory_records() -> Vec<TraceRecord> {
    [
        TraceRecordTemplate {
            sequence: "01",
            component: TraceComponent::Provider,
            operation: TraceOperation::ProviderRoute,
            decision: Some(TraceDecision::ProviderRoute),
            outcome: TraceOutcome::Noop,
            reason_code: "provider_candidate",
        },
        TraceRecordTemplate {
            sequence: "02",
            component: TraceComponent::Calendar,
            operation: TraceOperation::CalendarDryRun,
            decision: Some(TraceDecision::LifecycleUpdated),
            outcome: TraceOutcome::DryRun,
            reason_code: "calendar_dry_run",
        },
        TraceRecordTemplate {
            sequence: "03",
            component: TraceComponent::Correction,
            operation: TraceOperation::UserCorrection,
            decision: Some(TraceDecision::UserCorrected),
            outcome: TraceOutcome::Noop,
            reason_code: "user_correction_applied",
        },
        TraceRecordTemplate {
            sequence: "04",
            component: TraceComponent::Replay,
            operation: TraceOperation::ReplayRun,
            decision: Some(TraceDecision::ReplayCompared),
            outcome: TraceOutcome::ReplayRecorded,
            reason_code: "replay_run",
        },
    ]
    .into_iter()
    .map(record)
    .collect()
}

pub fn report_with_step(record: &TraceRecord) -> TrajectoryAnchorReport {
    TrajectoryAnchorReport {
        trajectory_case_id: CASE_ID.to_owned(),
        steps: vec![anchor_step(record)],
    }
}

pub fn provider_route_record() -> TraceRecord {
    trajectory_records().remove(0)
}

pub fn assert_serialized_report_is_sanitized(serialized: &str) -> Result<(), String> {
    for forbidden in "raw_title|full_message|provider_json|native_identifier|Provider meeting|private title|private message|MORROW_PRIVACY_CANARY".split('|') {
        if serialized.contains(forbidden) {
            return Err(format!(
                "trajectory anchor report leaked forbidden token {forbidden}"
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct TraceRecordTemplate {
    sequence: &'static str,
    component: TraceComponent,
    operation: TraceOperation,
    decision: Option<TraceDecision>,
    outcome: TraceOutcome,
    reason_code: &'static str,
}

fn record(template: TraceRecordTemplate) -> TraceRecord {
    TraceRecord {
        schema_version: TraceSchemaVersion::V1,
        trace: TraceIds {
            trace_id: format!("trace_000000000000000000000000000000{}", template.sequence),
            span_id: format!("span_000000000000000000000000000000{}", template.sequence),
            parent_span_id: None,
            chat_hash: Some(
                "sha256:66e0bc3220b7dd3d0651965d244ebba7f5a8ae571be6874570b58495cdf26d85"
                    .to_owned(),
            ),
            message_hash: Some(
                "sha256:d9cee5362324ef2404962c149f0c564c7f6f009fe91ba5985cc5341a79a348de"
                    .to_owned(),
            ),
        },
        span: TraceSpan {
            component: template.component,
            operation: template.operation,
            decision: template.decision,
            outcome: template.outcome,
            started_at: format!("2026-07-05T00:00:{}Z", template.sequence),
            ended_at: Some(format!("2026-07-05T00:00:{}Z", template.sequence)),
            provider_id: None,
            model_id: None,
            template_version: None,
            reason_code: Some(template.reason_code.to_owned()),
            confidence_millis: Some(860),
            title_hash: Some(
                "sha256:b12b98bbd7274cb93dee7cb18ca8a9a0247a007c99480b46f1879cc211459c0d"
                    .to_owned(),
            ),
            title_status: Some("hashed".to_owned()),
            privacy_tier: TracePrivacyTier::HashedIdentifier,
            classifier_stage: None,
            router_stage: None,
            ood_score_millis: None,
            replay_run_id: Some("phase5-trajectory-local-001".to_owned()),
        },
    }
}

pub fn anchor_step(record: &TraceRecord) -> TrajectoryAnchorStep {
    TrajectoryAnchorStep::from_trace_record(TrajectoryAnchorStepInput {
        trajectory_case_id: CASE_ID,
        feature_snapshot_id: FEATURE_SNAPSHOT_ID,
        label_type: "proposal_outcome",
        label_value: "accepted",
        decision_evidence_subject_id: DECISION_EVIDENCE_SUBJECT_ID,
        record,
    })
}
