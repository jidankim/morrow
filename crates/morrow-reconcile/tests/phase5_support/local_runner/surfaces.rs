use std::error::Error;

use morrow_reconcile::{
    apply_reconciliation, reconcile_candidate, CandidateLifecycle, ExternalItemObservation,
};
use morrow_storage::{
    CandidateDraft, CandidateId, CandidateKind, CandidateState, DiagnosticsTraceLinkage,
    ExternalObjectMapping, ExternalSource, FeatureSnapshot, FeedbackLabelSource, FeedbackLabelType,
    FeedbackLabelValue, FeedbackPrivacyTier, FeedbackRecordMeta, FeedbackSourceExcerptPolicy,
    FeedbackSubjectType, Label, QuietLogDraft, Store, FEEDBACK_EVAL_SCHEMA_VERSION,
};

use super::artifact::CaseRunResult;
use crate::phase5_support::schema::{ExpectedOutcome, ProposalKindExpectation, TrajectoryCase};

type SurfaceResult<T> = Result<T, Box<dyn Error>>;

pub(super) fn run_case(
    store: &Store,
    case: &TrajectoryCase,
    index: usize,
) -> SurfaceResult<CaseRunResult> {
    let candidate_id = match case.expected_proposal_kind {
        ProposalKindExpectation::NoProposal => {
            record_quiet_surface(store, case, index)?;
            None
        }
        ProposalKindExpectation::CalendarEvent | ProposalKindExpectation::TaskReminder => {
            Some(run_lifecycle_surface(store, case, index)?)
        }
    };
    record_eval_surface(store, case, candidate_id.as_ref(), index)?;
    let cleanup = case.cleanup.ok_or_else(|| {
        std::io::Error::other(format!(
            "case {} missing cleanup expectation",
            case.trajectory_case_id
        ))
    })?;
    Ok(CaseRunResult {
        trajectory_case_id: case.trajectory_case_id.to_owned(),
        expected_outcomes: case
            .expected_outcomes
            .iter()
            .map(|outcome| outcome.as_str().to_owned())
            .collect(),
        expected_labels: case
            .expected_labels
            .iter()
            .map(|label| format!("{}={}", label.label_type, label.label_value))
            .collect(),
        trace_operations: case
            .expected_trace_operations
            .iter()
            .map(|operation| format!("{}:{}", operation.component, operation.operation))
            .collect(),
        local_fake_scan: true,
        proposal_lifecycle: true,
        lifecycle_replay: case
            .expected_outcomes
            .contains(&ExpectedOutcome::ReplayIdempotent),
        phase4_approval_correction: phase4_observation(case).to_owned(),
        diagnostics_surface: true,
        decision_evidence_surface: true,
        cleanup_action: cleanup.action.as_str().to_owned(),
    })
}

fn run_lifecycle_surface(
    store: &Store,
    case: &TrajectoryCase,
    index: usize,
) -> SurfaceResult<CandidateId> {
    let candidate_id = create_visible_candidate(store, case, index)?;
    let plan = reconcile_candidate(
        &CandidateLifecycle {
            candidate_id: candidate_id.clone(),
            kind: case.expected_candidate_kind,
            state: CandidateState::Visible,
            mapping: Some(mapping_for(&candidate_id, case)),
            observed_at: observed_at(index),
        },
        &phase4_external_observation(case, index),
    )?;
    apply_reconciliation(store, &plan)?;
    if case
        .expected_outcomes
        .contains(&ExpectedOutcome::ReplayIdempotent)
    {
        apply_reconciliation(store, &plan)?;
    }
    Ok(candidate_id)
}

fn create_visible_candidate(
    store: &Store,
    case: &TrajectoryCase,
    index: usize,
) -> SurfaceResult<CandidateId> {
    let candidate_id = store.create_candidate(CandidateDraft {
        kind: case.expected_candidate_kind,
        chat_guid: format!("phase5-chat-{index}"),
        anchor_message_guid: format!("phase5-message-{index}"),
        title: format!("Phase5 local fixture {index}"),
        confidence_millis: 900,
        normalized_time: format!("2026-07-{:02}T10:00:00Z", index + 1),
        evidence_excerpt: "sanitized fixture excerpt".to_owned(),
        observed_at: observed_at(index),
    })?;
    store.transition_candidate(
        &candidate_id,
        CandidateState::CreatingExternal,
        "phase5_local_fake_scan",
        observed_at(index) + 1,
    )?;
    store.transition_candidate(
        &candidate_id,
        CandidateState::Visible,
        "phase5_local_fake_proposal",
        observed_at(index) + 2,
    )?;
    Ok(candidate_id)
}

fn record_quiet_surface(store: &Store, case: &TrajectoryCase, index: usize) -> SurfaceResult<()> {
    store.record_quiet_log(QuietLogDraft {
        chat_guid: format!("phase5-quiet-chat-{index}"),
        anchor_message_guid: format!("phase5-quiet-message-{index}"),
        reason: case.family.to_owned(),
        excerpt: "sanitized quiet placeholder".to_owned(),
        provider_diagnostic: Some("local_provider_disabled".to_owned()),
        created_at: observed_at(index),
    })?;
    Ok(())
}

fn record_eval_surface(
    store: &Store,
    case: &TrajectoryCase,
    candidate_id: Option<&CandidateId>,
    index: usize,
) -> SurfaceResult<()> {
    let meta = feedback_meta(case, candidate_id.cloned(), index);
    store.record_feature_snapshot(FeatureSnapshot {
        id: None,
        snapshot_key: snapshot_key(case),
        meta: meta.clone(),
        route: Some(route_for(case).to_owned()),
        reason_code: Some(case.family.to_owned()),
        confidence_millis: Some(900),
        participant_count: Some(2),
        tapback_signal: Some("absent".to_owned()),
        sender_signal_available: true,
        context_window_available: true,
        excerpt: Some("Source excerpt hidden by settings.".to_owned()),
    })?;
    for label in case.expected_labels {
        let label_type = FeedbackLabelType::parse(label.label_type)?;
        store.record_label(Label {
            id: None,
            label_key: format!("{}-{}", snapshot_key(case), label.label_type),
            label_value: FeedbackLabelValue::parse(label_type, label.label_value, "label_value")?,
            meta: meta.clone(),
        })?;
    }
    Ok(())
}

fn feedback_meta(
    case: &TrajectoryCase,
    candidate_id: Option<CandidateId>,
    index: usize,
) -> FeedbackRecordMeta {
    FeedbackRecordMeta {
        schema_version: FEEDBACK_EVAL_SCHEMA_VERSION,
        subject_type: candidate_id
            .as_ref()
            .map_or(FeedbackSubjectType::QuietLog, |_| {
                FeedbackSubjectType::Candidate
            }),
        subject_id: subject_id(case),
        candidate_id,
        chat_guid: format!("phase5-feedback-chat-{index}"),
        anchor_message_guid: format!("phase5-feedback-message-{index}"),
        diagnostics: DiagnosticsTraceLinkage {
            trace_id: Some(format!("trace_{:032x}", index + 1)),
            span_id: Some(format!("span_{:032x}", index + 101)),
            parent_span_id: None,
            chat_hash: Some(format!("sha256:{:064x}", index + 201)),
            message_hash: Some(format!("sha256:{:064x}", index + 301)),
        },
        provider_model_prompt_version_id: None,
        source_excerpt_policy: FeedbackSourceExcerptPolicy::Hide,
        label_source: FeedbackLabelSource::EvalRunner,
        privacy_tier: FeedbackPrivacyTier::InternalMetadata,
        privacy_metadata_json: "{}".to_owned(),
        created_at: observed_at(index),
        expires_at: None,
    }
}

fn mapping_for(candidate_id: &CandidateId, case: &TrajectoryCase) -> ExternalObjectMapping {
    ExternalObjectMapping {
        candidate_id: candidate_id.clone(),
        source: external_source(case.expected_candidate_kind),
        external_object_id: format!("phase5-proposed-{}", subject_id(case)),
        external_source_id: "phase5-local-source".to_owned(),
        mapped_at: 1_783_000_000,
    }
}

const fn external_source(kind: CandidateKind) -> ExternalSource {
    match kind {
        CandidateKind::TaskReminder
        | CandidateKind::ReminderUpdate
        | CandidateKind::ReminderReschedule
        | CandidateKind::ReminderCancellation => ExternalSource::Reminders,
        CandidateKind::CalendarEvent
        | CandidateKind::EventUpdate
        | CandidateKind::EventReschedule
        | CandidateKind::EventCancellation => ExternalSource::Calendar,
    }
}

fn phase4_external_observation(case: &TrajectoryCase, index: usize) -> ExternalItemObservation {
    match case.family {
        "scheduled_meeting_rejected" => ExternalItemObservation::DeletedFromProposed,
        "task_reminder_rejected" => ExternalItemObservation::Completed,
        "scheduled_meeting_edited_before_approval" => ExternalItemObservation::PendingEdited {
            observed_title: Some("sanitized edited placeholder".to_owned()),
            observed_normalized_time: Some("2026-07-15T11:30:00Z".to_owned()),
        },
        _ => ExternalItemObservation::approved_by_move(
            &format!("phase5-approved-local-{index}"),
            "phase5-source",
        ),
    }
}

fn phase4_observation(case: &TrajectoryCase) -> &'static str {
    match case.family {
        "scheduled_meeting_rejected" | "task_reminder_rejected" => "rejected_observed",
        "scheduled_meeting_edited_before_approval" => "pending_edited",
        "provider_quiet_low_confidence" | "privacy_canary_rejection" => "blocked_before_mutation",
        _ => "accepted",
    }
}

fn route_for(case: &TrajectoryCase) -> &'static str {
    match case.expected_proposal_kind {
        ProposalKindExpectation::NoProposal => "quiet_stop",
        ProposalKindExpectation::CalendarEvent | ProposalKindExpectation::TaskReminder => {
            "deterministic_candidate"
        }
    }
}

fn subject_id(case: &TrajectoryCase) -> String {
    case.trajectory_case_id.replace(':', "-")
}

fn snapshot_key(case: &TrajectoryCase) -> String {
    format!("{}-snapshot", subject_id(case))
}

fn observed_at(index: usize) -> i64 {
    1_783_000_000 + i64::try_from(index).unwrap_or(0)
}
