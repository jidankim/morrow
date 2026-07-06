use morrow_diagnostics::{
    TraceComponent, TraceDecision, TraceIds, TraceOperation, TraceOutcome, TracePrivacyTier,
    TraceRecord, TraceSchemaVersion, TraceSpan,
};
use morrow_storage::{
    CandidateDraft, CandidateId, CandidateKind, DiagnosticsTraceLinkage, FeatureSnapshot,
    FeedbackLabelSource, FeedbackLabelValue, FeedbackPrivacyTier, FeedbackRecordMeta,
    FeedbackSourceExcerptPolicy, FeedbackSubjectType, ProposalOutcomeLabel, Store,
    FEEDBACK_EVAL_SCHEMA_VERSION,
};

pub const CASE_ID: &str = "phase5-case-calendar-accepted";
pub const FEATURE_SNAPSHOT_ID: &str = "phase5-case-calendar-accepted-snapshot";
pub const TRACE_ID: &str = "trace_00000000000000000000000000000500";
pub const ANCHOR_SPAN_ID: &str = "span_00000000000000000000000000000501";

pub fn record_trajectory_evidence(store_path: &std::path::Path) -> Result<CandidateId, String> {
    let store = Store::open(store_path).map_err(|error| error.to_string())?;
    let candidate_id = store
        .create_candidate(CandidateDraft {
            kind: CandidateKind::CalendarEvent,
            chat_guid: "chat-guid-private".to_owned(),
            anchor_message_guid: "message-guid-private".to_owned(),
            title: "private trajectory title".to_owned(),
            confidence_millis: 860,
            normalized_time: "2026-07-15T00:00:00Z".to_owned(),
            evidence_excerpt: "private message excerpt".to_owned(),
            observed_at: 1_783_000_000,
        })
        .map_err(|error| error.to_string())?;
    let meta = trajectory_meta(candidate_id.as_str(), &candidate_id);
    store
        .record_label(morrow_storage::Label {
            id: None,
            label_key: "phase5-case-calendar-accepted-label".to_owned(),
            label_value: FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::Accepted),
            meta: meta.clone(),
        })
        .map_err(|error| error.to_string())?;
    store
        .record_feature_snapshot(FeatureSnapshot {
            id: None,
            snapshot_key: FEATURE_SNAPSHOT_ID.to_owned(),
            meta,
            route: Some("provider_candidate".to_owned()),
            reason_code: Some("provider_valid".to_owned()),
            confidence_millis: Some(860),
            participant_count: Some(2),
            tapback_signal: Some("absent".to_owned()),
            sender_signal_available: false,
            context_window_available: false,
            excerpt: Some("Source excerpt hidden by settings.".to_owned()),
        })
        .map_err(|error| error.to_string())?;
    Ok(candidate_id)
}

pub fn trajectory_records() -> Vec<TraceRecord> {
    vec![
        provider_route_record(),
        calendar_dry_run_record(),
        user_correction_record(),
        replay_run_record(),
    ]
}

pub fn provider_route_record() -> TraceRecord {
    record(TraceRecordTemplate {
        span_id: ANCHOR_SPAN_ID,
        component: TraceComponent::Provider,
        operation: TraceOperation::ProviderRoute,
        decision: Some(TraceDecision::ProviderRoute),
        outcome: TraceOutcome::Noop,
        reason_code: "provider_candidate",
        second: "01",
    })
}

pub fn trace_jsonl(records: &[TraceRecord]) -> Result<String, String> {
    let lines = records
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(lines.join("\n") + "\n")
}

fn trajectory_meta(subject_id: &str, candidate_id: &CandidateId) -> FeedbackRecordMeta {
    FeedbackRecordMeta {
        schema_version: FEEDBACK_EVAL_SCHEMA_VERSION,
        subject_type: FeedbackSubjectType::Candidate,
        subject_id: subject_id.to_owned(),
        candidate_id: Some(candidate_id.clone()),
        chat_guid: "chat-guid-private".to_owned(),
        anchor_message_guid: "message-guid-private".to_owned(),
        diagnostics: DiagnosticsTraceLinkage {
            trace_id: Some(TRACE_ID.to_owned()),
            span_id: Some(ANCHOR_SPAN_ID.to_owned()),
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
        provider_model_prompt_version_id: None,
        source_excerpt_policy: FeedbackSourceExcerptPolicy::Hide,
        label_source: FeedbackLabelSource::EvalRunner,
        privacy_tier: FeedbackPrivacyTier::HashedIdentifier,
        privacy_metadata_json: "{}".to_owned(),
        created_at: 1_783_000_000,
        expires_at: Some(1_785_592_000),
    }
}

fn calendar_dry_run_record() -> TraceRecord {
    record(TraceRecordTemplate {
        span_id: "span_00000000000000000000000000000502",
        component: TraceComponent::Calendar,
        operation: TraceOperation::CalendarDryRun,
        decision: Some(TraceDecision::LifecycleUpdated),
        outcome: TraceOutcome::DryRun,
        reason_code: "calendar_dry_run",
        second: "02",
    })
}

fn user_correction_record() -> TraceRecord {
    record(TraceRecordTemplate {
        span_id: "span_00000000000000000000000000000503",
        component: TraceComponent::Correction,
        operation: TraceOperation::UserCorrection,
        decision: Some(TraceDecision::UserCorrected),
        outcome: TraceOutcome::Noop,
        reason_code: "user_correction_applied",
        second: "03",
    })
}

fn replay_run_record() -> TraceRecord {
    record(TraceRecordTemplate {
        span_id: "span_00000000000000000000000000000504",
        component: TraceComponent::Replay,
        operation: TraceOperation::ReplayRun,
        decision: Some(TraceDecision::ReplayCompared),
        outcome: TraceOutcome::ReplayRecorded,
        reason_code: "replay_run",
        second: "04",
    })
}

#[derive(Debug, Clone, Copy)]
struct TraceRecordTemplate {
    span_id: &'static str,
    component: TraceComponent,
    operation: TraceOperation,
    decision: Option<TraceDecision>,
    outcome: TraceOutcome,
    reason_code: &'static str,
    second: &'static str,
}

fn record(template: TraceRecordTemplate) -> TraceRecord {
    TraceRecord {
        schema_version: TraceSchemaVersion::V1,
        trace: TraceIds {
            trace_id: TRACE_ID.to_owned(),
            span_id: template.span_id.to_owned(),
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
            started_at: format!("2026-07-05T00:00:{}Z", template.second),
            ended_at: Some(format!("2026-07-05T00:00:{}Z", template.second)),
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
