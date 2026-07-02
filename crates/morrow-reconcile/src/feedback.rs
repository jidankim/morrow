use morrow_storage::{
    CandidateFeedbackContext, DiagnosticsTraceLinkage, FeedbackEvent, FeedbackEventType,
    FeedbackLabelSource, FeedbackLabelValue, FeedbackPrivacyTier, FeedbackRecordMeta,
    FeedbackSourceExcerptPolicy, FeedbackSubjectType, FieldQualityLabel, Label,
    ProposalOutcomeLabel, Store, SystemOutcomeLabel, FEEDBACK_EVAL_SCHEMA_VERSION,
};

use crate::{
    LifecycleAction, LifecycleReason, PendingEditFeedback, ReconcileError, ReconciliationPlan,
    Suppression,
};

pub(crate) fn record_lifecycle_feedback(
    store: &Store,
    plan: &ReconciliationPlan,
) -> Result<(), ReconcileError> {
    let context = feedback_context(store, plan)?;
    for action in &plan.actions {
        if let LifecycleAction::Transition {
            reason,
            observed_at,
            ..
        } = action
        {
            record_reason_feedback(store, &context, *reason, *observed_at)?;
        }
    }
    if plan
        .suppressions
        .contains(&Suppression::EditedPendingNoOverwrite)
    {
        if let Some(feedback) = plan.pending_edit_feedback {
            record_pending_edit_feedback(store, &context, feedback)?;
        }
    }
    Ok(())
}

fn feedback_context(
    store: &Store,
    plan: &ReconciliationPlan,
) -> Result<CandidateFeedbackContext, ReconcileError> {
    let records_feedback = plan.actions.iter().any(|action| match action {
        LifecycleAction::Transition { reason, .. } => reason_feedback_event(*reason).is_some(),
        LifecycleAction::CreateExternalProposal
        | LifecycleAction::UpsertExternalMapping(_)
        | LifecycleAction::CleanupProposedExternal { .. } => false,
    }) || plan
        .suppressions
        .contains(&Suppression::EditedPendingNoOverwrite);
    if records_feedback {
        Ok(store.candidate_feedback_context(&plan.candidate_id)?)
    } else {
        Ok(CandidateFeedbackContext {
            candidate_id: plan.candidate_id.clone(),
            chat_guid: String::new(),
            anchor_message_guid: String::new(),
        })
    }
}

fn record_reason_feedback(
    store: &Store,
    context: &CandidateFeedbackContext,
    reason: LifecycleReason,
    observed_at: i64,
) -> Result<(), ReconcileError> {
    let Some(event_type) = reason_feedback_event(reason) else {
        return Ok(());
    };
    record_event(store, context, event_type, observed_at)?;
    match reason {
        LifecycleReason::ApprovedByMove | LifecycleReason::ApprovedByCopyCleanup => record_label(
            store,
            context,
            FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::Accepted),
            observed_at,
        ),
        LifecycleReason::RejectedByDelete | LifecycleReason::ProposedReminderCompletedResolved => {
            record_label(
                store,
                context,
                FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::RejectedObserved),
                observed_at,
            )
        }
        LifecycleReason::UnknownDisappearance => record_label(
            store,
            context,
            FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::Unknown),
            observed_at,
        ),
        LifecycleReason::ExternalCreationFailed => record_label(
            store,
            context,
            FeedbackLabelValue::SystemOutcome(SystemOutcomeLabel::FailedExternalCreation),
            observed_at,
        ),
        LifecycleReason::CompletedClosure
        | LifecycleReason::PartialWriteRecovered
        | LifecycleReason::ManualChangeProposalOnly
        | LifecycleReason::CandidateSuperseded
        | LifecycleReason::CandidateRescheduled
        | LifecycleReason::CandidateCancelled
        | LifecycleReason::CreatingExternalProposal => Ok(()),
    }
}

const fn reason_feedback_event(reason: LifecycleReason) -> Option<FeedbackEventType> {
    match reason {
        LifecycleReason::ApprovedByMove => Some(FeedbackEventType::ApprovedByMove),
        LifecycleReason::ApprovedByCopyCleanup => Some(FeedbackEventType::ApprovedByCopy),
        LifecycleReason::RejectedByDelete => Some(FeedbackEventType::RejectedByDelete),
        LifecycleReason::UnknownDisappearance => Some(FeedbackEventType::UnknownDisappearance),
        LifecycleReason::ProposedReminderCompletedResolved => {
            Some(FeedbackEventType::ProposedReminderCompletedResolved)
        }
        LifecycleReason::ExternalCreationFailed => Some(FeedbackEventType::ExternalCreationFailed),
        LifecycleReason::CompletedClosure
        | LifecycleReason::PartialWriteRecovered
        | LifecycleReason::ManualChangeProposalOnly
        | LifecycleReason::CandidateSuperseded
        | LifecycleReason::CandidateRescheduled
        | LifecycleReason::CandidateCancelled
        | LifecycleReason::CreatingExternalProposal => None,
    }
}

fn record_pending_edit_feedback(
    store: &Store,
    context: &CandidateFeedbackContext,
    feedback: PendingEditFeedback,
) -> Result<(), ReconcileError> {
    record_event(
        store,
        context,
        FeedbackEventType::PendingEdited,
        feedback.observed_at,
    )?;
    record_label(
        store,
        context,
        FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::PendingEdited),
        feedback.observed_at,
    )?;
    if feedback.title_edited {
        record_label(
            store,
            context,
            FeedbackLabelValue::FieldQuality(FieldQualityLabel::TitleEdited),
            feedback.observed_at,
        )?;
    }
    if feedback.time_edited {
        record_label(
            store,
            context,
            FeedbackLabelValue::FieldQuality(FieldQualityLabel::TimeEdited),
            feedback.observed_at,
        )?;
    }
    if !feedback.title_edited && !feedback.time_edited {
        record_label(
            store,
            context,
            FeedbackLabelValue::FieldQuality(FieldQualityLabel::Unknown),
            feedback.observed_at,
        )?;
    }
    Ok(())
}

fn record_event(
    store: &Store,
    context: &CandidateFeedbackContext,
    event_type: FeedbackEventType,
    observed_at: i64,
) -> Result<(), ReconcileError> {
    store.record_feedback_event(FeedbackEvent {
        id: None,
        event_key: feedback_key(context, "event", event_type.as_str()),
        event_type,
        meta: meta(context, observed_at),
    })?;
    Ok(())
}

fn record_label(
    store: &Store,
    context: &CandidateFeedbackContext,
    label_value: FeedbackLabelValue,
    observed_at: i64,
) -> Result<(), ReconcileError> {
    let label_type = label_value.label_type();
    store.record_label(Label {
        id: None,
        label_key: feedback_key(context, label_type.as_str(), label_value.as_str()),
        label_value,
        meta: meta(context, observed_at),
    })?;
    Ok(())
}

fn meta(context: &CandidateFeedbackContext, observed_at: i64) -> FeedbackRecordMeta {
    FeedbackRecordMeta {
        schema_version: FEEDBACK_EVAL_SCHEMA_VERSION,
        subject_type: FeedbackSubjectType::Candidate,
        subject_id: context.candidate_id.to_string(),
        candidate_id: Some(context.candidate_id.clone()),
        chat_guid: context.chat_guid.clone(),
        anchor_message_guid: context.anchor_message_guid.clone(),
        diagnostics: DiagnosticsTraceLinkage {
            trace_id: None,
            span_id: None,
            parent_span_id: None,
            chat_hash: None,
            message_hash: None,
        },
        provider_model_prompt_version_id: None,
        source_excerpt_policy: FeedbackSourceExcerptPolicy::Hide,
        label_source: FeedbackLabelSource::Lifecycle,
        privacy_tier: FeedbackPrivacyTier::InternalMetadata,
        privacy_metadata_json: "{}".to_owned(),
        created_at: observed_at,
        expires_at: None,
    }
}

fn feedback_key(context: &CandidateFeedbackContext, kind: &str, value: &str) -> String {
    format!("{}:lifecycle:{kind}:{value}", context.candidate_id.as_str())
}
