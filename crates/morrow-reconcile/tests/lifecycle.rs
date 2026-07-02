//! Reconciliation lifecycle regression tests.

use morrow_reconcile::{
    apply_reconciliation, reconcile_candidate, CandidateLifecycle, DisappearanceEvidence,
    ExternalItemObservation, LifecycleAction, LifecycleReason, Suppression,
};
use morrow_storage::{CandidateKind, CandidateState, ExternalSource, Store};

/// Shared lifecycle test fixtures.
pub mod support;
use support::{
    assert_audit_reason, creating_candidate, db_path, draft, mapping, mapping_with_source,
    visible_candidate, visible_lifecycle, TestError,
};

#[test]
fn characterizes_storage_transition_rules_when_wrapped_by_reconciliation() -> Result<(), TestError>
{
    let store = Store::open(&db_path("characterizes_storage_transition_rules"))?;
    let candidate_id = store.create_candidate(draft(CandidateKind::CalendarEvent, "baseline"))?;

    let err = store
        .transition_candidate(&candidate_id, CandidateState::Visible, "direct_visible", 2)
        .err()
        .ok_or(TestError::MissingExpectedError)?;

    assert!(matches!(
        err,
        morrow_storage::StorageError::InvalidTransition { .. }
    ));
    Ok(())
}

#[test]
fn reconciles_visible_candidate_lifecycle_when_external_item_changes() -> Result<(), TestError> {
    let store = Store::open(&db_path("reconciles_visible_candidate_lifecycle"))?;

    let moved = visible_candidate(&store, CandidateKind::CalendarEvent, "move")?;
    let moved_plan = reconcile_candidate(
        &CandidateLifecycle {
            candidate_id: moved.clone(),
            kind: CandidateKind::CalendarEvent,
            state: CandidateState::Visible,
            mapping: Some(mapping(&moved, "proposed-move")),
            observed_at: 20,
        },
        &ExternalItemObservation::approved_by_move("real-calendar", "source-main"),
    )?;
    apply_reconciliation(&store, &moved_plan)?;
    apply_reconciliation(&store, &moved_plan)?;
    assert_audit_reason(&store, &moved, LifecycleReason::ApprovedByMove)?;

    let copied = visible_candidate(&store, CandidateKind::CalendarEvent, "copy")?;
    let copied_plan = reconcile_candidate(
        &CandidateLifecycle {
            candidate_id: copied.clone(),
            kind: CandidateKind::CalendarEvent,
            state: CandidateState::Visible,
            mapping: Some(mapping(&copied, "proposed-copy")),
            observed_at: 30,
        },
        &ExternalItemObservation::approved_by_copy("real-copy", "source-main"),
    )?;
    assert!(copied_plan
        .actions
        .contains(&LifecycleAction::CleanupProposedExternal {
            external_object_id: "proposed-copy".to_owned(),
            source: ExternalSource::Calendar,
        }));
    apply_reconciliation(&store, &copied_plan)?;
    assert_audit_reason(&store, &copied, LifecycleReason::ApprovedByCopyCleanup)?;

    let deleted = visible_candidate(&store, CandidateKind::CalendarEvent, "delete")?;
    let deleted_plan = reconcile_candidate(
        &CandidateLifecycle {
            candidate_id: deleted.clone(),
            kind: CandidateKind::CalendarEvent,
            state: CandidateState::Visible,
            mapping: Some(mapping(&deleted, "proposed-delete")),
            observed_at: 40,
        },
        &ExternalItemObservation::DeletedFromProposed,
    )?;
    apply_reconciliation(&store, &deleted_plan)?;
    assert_audit_reason(&store, &deleted, LifecycleReason::RejectedByDelete)?;

    let unknown = visible_candidate(&store, CandidateKind::CalendarEvent, "unknown")?;
    let unknown_plan = reconcile_candidate(
        &CandidateLifecycle {
            candidate_id: unknown.clone(),
            kind: CandidateKind::CalendarEvent,
            state: CandidateState::Visible,
            mapping: Some(mapping(&unknown, "proposed-unknown")),
            observed_at: 50,
        },
        &ExternalItemObservation::Disappeared {
            evidence: DisappearanceEvidence::StoreResetOrPermissionGap,
        },
    )?;
    apply_reconciliation(&store, &unknown_plan)?;
    assert_audit_reason(&store, &unknown, LifecycleReason::UnknownDisappearance)?;

    Ok(())
}

#[test]
fn suppresses_edited_pending_and_manual_change_proposals_without_overwriting(
) -> Result<(), TestError> {
    let edited = visible_lifecycle(CandidateKind::TaskReminder, CandidateState::Visible);
    let edited_plan = reconcile_candidate(
        &edited,
        &ExternalItemObservation::PendingEdited {
            observed_title: Some("user edited title".to_owned()),
            observed_normalized_time: None,
        },
    )?;

    assert!(edited_plan.actions.is_empty());
    assert!(edited_plan
        .suppressions
        .contains(&Suppression::EditedPendingNoOverwrite));

    let update = visible_lifecycle(CandidateKind::EventUpdate, CandidateState::Queued);
    let update_plan = reconcile_candidate(&update, &ExternalItemObservation::Pending)?;

    assert!(update_plan
        .suppressions
        .contains(&Suppression::ManualChangeProposalOnly));
    assert!(update_plan.actions.contains(&LifecycleAction::Transition {
        to: CandidateState::Suppressed,
        reason: LifecycleReason::ManualChangeProposalOnly,
        observed_at: 10,
    }));
    Ok(())
}

#[test]
fn closes_approved_reminder_when_completed_and_ignores_completed_candidates(
) -> Result<(), TestError> {
    let store = Store::open(&db_path("closes_approved_reminder"))?;
    let approved = visible_candidate(&store, CandidateKind::TaskReminder, "complete")?;
    store.transition_candidate(&approved, CandidateState::Approved, "setup_approved", 12)?;

    let completed_plan = reconcile_candidate(
        &CandidateLifecycle {
            candidate_id: approved.clone(),
            kind: CandidateKind::TaskReminder,
            state: CandidateState::Approved,
            mapping: Some(mapping_with_source(
                &approved,
                ExternalSource::Reminders,
                "real-reminder",
            )),
            observed_at: 60,
        },
        &ExternalItemObservation::Completed,
    )?;
    apply_reconciliation(&store, &completed_plan)?;
    assert_audit_reason(&store, &approved, LifecycleReason::CompletedClosure)?;

    let closed_plan = reconcile_candidate(
        &visible_lifecycle(CandidateKind::TaskReminder, CandidateState::Completed),
        &ExternalItemObservation::approved_by_move("changed-after-complete", "source-main"),
    )?;
    assert!(closed_plan.actions.is_empty());
    assert!(closed_plan
        .suppressions
        .contains(&Suppression::ClosedCandidate));
    Ok(())
}

#[test]
fn recovers_partial_external_creation_without_duplicate_proposals() -> Result<(), TestError> {
    let store = Store::open(&db_path("recovers_partial_external_creation"))?;
    let recovered = creating_candidate(&store, CandidateKind::CalendarEvent, "restart-visible")?;
    let recovered_mapping = mapping(&recovered, "external-after-restart");
    store.upsert_external_mapping(recovered_mapping.clone())?;

    let recovered_plan = reconcile_candidate(
        &CandidateLifecycle {
            candidate_id: recovered.clone(),
            kind: CandidateKind::CalendarEvent,
            state: CandidateState::CreatingExternal,
            mapping: Some(recovered_mapping),
            observed_at: 70,
        },
        &ExternalItemObservation::Pending,
    )?;

    assert!(!recovered_plan
        .actions
        .contains(&LifecycleAction::CreateExternalProposal));
    apply_reconciliation(&store, &recovered_plan)?;
    assert_audit_reason(&store, &recovered, LifecycleReason::PartialWriteRecovered)?;

    let failed = creating_candidate(&store, CandidateKind::CalendarEvent, "restart-missing")?;
    let failed_plan = reconcile_candidate(
        &CandidateLifecycle {
            candidate_id: failed.clone(),
            kind: CandidateKind::CalendarEvent,
            state: CandidateState::CreatingExternal,
            mapping: None,
            observed_at: 80,
        },
        &ExternalItemObservation::CreationFailed,
    )?;
    apply_reconciliation(&store, &failed_plan)?;
    assert_audit_reason(&store, &failed, LifecycleReason::ExternalCreationFailed)?;
    Ok(())
}

#[test]
fn covers_every_candidate_state_when_reconciliation_runs() -> Result<(), TestError> {
    let queued = reconcile_candidate(
        &visible_lifecycle(CandidateKind::CalendarEvent, CandidateState::Queued),
        &ExternalItemObservation::Pending,
    )?;
    assert!(queued.actions.contains(&LifecycleAction::Transition {
        to: CandidateState::CreatingExternal,
        reason: LifecycleReason::CreatingExternalProposal,
        observed_at: 10,
    }));

    let creating = reconcile_candidate(
        &visible_lifecycle(
            CandidateKind::CalendarEvent,
            CandidateState::CreatingExternal,
        ),
        &ExternalItemObservation::CreationFailed,
    )?;
    assert!(creating.actions.contains(&LifecycleAction::Transition {
        to: CandidateState::Failed,
        reason: LifecycleReason::ExternalCreationFailed,
        observed_at: 10,
    }));

    let visible = reconcile_candidate(
        &visible_lifecycle(CandidateKind::CalendarEvent, CandidateState::Visible),
        &ExternalItemObservation::DeletedFromProposed,
    )?;
    assert!(visible.actions.contains(&LifecycleAction::Transition {
        to: CandidateState::Rejected,
        reason: LifecycleReason::RejectedByDelete,
        observed_at: 10,
    }));

    let approved = reconcile_candidate(
        &visible_lifecycle(CandidateKind::CalendarEvent, CandidateState::Approved),
        &ExternalItemObservation::Completed,
    )?;
    assert!(approved.actions.contains(&LifecycleAction::Transition {
        to: CandidateState::Completed,
        reason: LifecycleReason::CompletedClosure,
        observed_at: 10,
    }));

    for terminal in [
        CandidateState::Completed,
        CandidateState::Rejected,
        CandidateState::Expired,
        CandidateState::Suppressed,
        CandidateState::Unknown,
        CandidateState::Failed,
    ] {
        let plan = reconcile_candidate(
            &visible_lifecycle(CandidateKind::CalendarEvent, terminal),
            &ExternalItemObservation::Pending,
        )?;
        assert!(plan.actions.is_empty());
        assert!(plan.suppressions.contains(&Suppression::ClosedCandidate));
    }
    Ok(())
}
