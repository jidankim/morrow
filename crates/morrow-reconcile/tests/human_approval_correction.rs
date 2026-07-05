#![doc = "Phase 4 human approval and correction lifecycle contract tests."]

use morrow_reconcile::{
    apply_reconciliation, reconcile_candidate, CandidateLifecycle, DisappearanceEvidence,
    ExternalItemObservation, LifecycleAction, LifecycleReason, Suppression,
};
use morrow_storage::{CandidateId, CandidateKind, CandidateState, ExternalSource, Store};

mod phase4_support;
#[doc = "Shared lifecycle test fixtures."]
pub mod support;
use phase4_support::{
    assert_audit_reason_count, assert_feedback_label_count, assert_feedback_text_absent,
    assert_invalid_observation, lifecycle, visible_plan,
};
use support::{db_path, mapping, mapping_with_source, visible_candidate, TestError};

#[test]
fn records_phase4_feedback_contract_when_user_approves_rejects_edits_or_external_creation_fails(
) -> Result<(), TestError> {
    let db_path = db_path("phase4_human_approval_contract");
    let store = Store::open(&db_path)?;

    let moved = visible_candidate(&store, CandidateKind::CalendarEvent, "phase4-move")?;
    let moved_plan = visible_plan(
        &moved,
        CandidateKind::CalendarEvent,
        Some(mapping(&moved, "phase4-proposed-move")),
        120,
        &ExternalItemObservation::approved_by_move("phase4-real-move", "source-main"),
    )?;
    apply_reconciliation(&store, &moved_plan)?;
    apply_reconciliation(&store, &moved_plan)?;

    let copied = visible_candidate(&store, CandidateKind::CalendarEvent, "phase4-copy")?;
    let copied_plan = visible_plan(
        &copied,
        CandidateKind::CalendarEvent,
        Some(mapping(&copied, "phase4-proposed-copy")),
        130,
        &ExternalItemObservation::approved_by_copy("phase4-real-copy", "source-main"),
    )?;
    assert!(copied_plan
        .actions
        .contains(&LifecycleAction::CleanupProposedExternal {
            external_object_id: "phase4-proposed-copy".to_owned(),
            source: ExternalSource::Calendar,
        }));
    apply_reconciliation(&store, &copied_plan)?;
    apply_reconciliation(&store, &copied_plan)?;

    let deleted = visible_candidate(&store, CandidateKind::CalendarEvent, "phase4-delete")?;
    let deleted_plan = visible_plan(
        &deleted,
        CandidateKind::CalendarEvent,
        Some(mapping(&deleted, "phase4-proposed-delete")),
        140,
        &ExternalItemObservation::DeletedFromProposed,
    )?;
    apply_reconciliation(&store, &deleted_plan)?;
    apply_reconciliation(&store, &deleted_plan)?;

    let completed = visible_candidate(&store, CandidateKind::TaskReminder, "phase4-complete")?;
    let completed_plan = visible_plan(
        &completed,
        CandidateKind::TaskReminder,
        Some(mapping_with_source(
            &completed,
            ExternalSource::Reminders,
            "phase4-proposed-complete",
        )),
        150,
        &ExternalItemObservation::Completed,
    )?;
    apply_reconciliation(&store, &completed_plan)?;
    apply_reconciliation(&store, &completed_plan)?;

    let unknown = visible_candidate(&store, CandidateKind::CalendarEvent, "phase4-unknown")?;
    let unknown_plan = visible_plan(
        &unknown,
        CandidateKind::CalendarEvent,
        Some(mapping(&unknown, "phase4-proposed-unknown")),
        160,
        &ExternalItemObservation::Disappeared {
            evidence: DisappearanceEvidence::StoreResetOrPermissionGap,
        },
    )?;
    apply_reconciliation(&store, &unknown_plan)?;
    apply_reconciliation(&store, &unknown_plan)?;

    let edited = visible_candidate(&store, CandidateKind::TaskReminder, "phase4-edit")?;
    let corrected_title = "raw user corrected phase4 title";
    let corrected_time = "2026-07-01T11:30:00Z";
    let edited_plan = visible_plan(
        &edited,
        CandidateKind::TaskReminder,
        Some(mapping_with_source(
            &edited,
            ExternalSource::Reminders,
            "phase4-proposed-edit",
        )),
        170,
        &ExternalItemObservation::PendingEdited {
            observed_title: Some(corrected_title.to_owned()),
            observed_normalized_time: Some(corrected_time.to_owned()),
        },
    )?;

    assert!(edited_plan.actions.is_empty(), "no overwrite");
    assert!(edited_plan
        .suppressions
        .contains(&Suppression::EditedPendingNoOverwrite));
    apply_reconciliation(&store, &edited_plan)?;
    apply_reconciliation(&store, &edited_plan)?;

    let failed = visible_candidate(&store, CandidateKind::CalendarEvent, "phase4-failed")?;
    let failed_plan = visible_plan(
        &failed,
        CandidateKind::CalendarEvent,
        Some(mapping(&failed, "phase4-proposed-failed")),
        180,
        &ExternalItemObservation::CreationFailed,
    )?;
    apply_reconciliation(&store, &failed_plan)?;
    apply_reconciliation(&store, &failed_plan)?;

    assert_audit_reason_count(&store, &moved, LifecycleReason::ApprovedByMove, 1)?;
    assert_audit_reason_count(&store, &copied, LifecycleReason::ApprovedByCopyCleanup, 1)?;
    assert_audit_reason_count(&store, &deleted, LifecycleReason::RejectedByDelete, 1)?;
    assert_audit_reason_count(
        &store,
        &completed,
        LifecycleReason::ProposedReminderCompletedResolved,
        1,
    )?;
    assert_audit_reason_count(&store, &unknown, LifecycleReason::UnknownDisappearance, 1)?;
    assert_audit_reason_count(&store, &failed, LifecycleReason::ExternalCreationFailed, 1)?;

    assert_feedback_label_count(&db_path, "proposal_outcome", "accepted", 2)?;
    assert_feedback_label_count(&db_path, "proposal_outcome", "rejected_observed", 2)?;
    assert_feedback_label_count(&db_path, "proposal_outcome", "pending_edited", 1)?;
    assert_feedback_label_count(&db_path, "proposal_outcome", "unknown", 1)?;
    assert_feedback_label_count(&db_path, "field_quality", "title_edited", 1)?;
    assert_feedback_label_count(&db_path, "field_quality", "time_edited", 1)?;
    assert_feedback_label_count(&db_path, "system_outcome", "failed_external_creation", 1)?;
    assert_feedback_text_absent(&db_path, corrected_title)?;
    assert_feedback_text_absent(&db_path, corrected_time)?;

    println!(
        "PASS named assertions: accepted rejected_observed pending_edited title_edited time_edited unknown failed_external_creation no overwrite"
    );
    Ok(())
}

#[test]
fn suppresses_approved_mutation_when_approved_real_item_changes() -> Result<(), TestError> {
    let approved = CandidateLifecycle {
        candidate_id: CandidateId::derive(
            CandidateKind::CalendarEvent,
            "phase4-approved-chat",
            "phase4-approved-message",
            "2026-07-01T10:00:00Z",
        ),
        kind: CandidateKind::CalendarEvent,
        state: CandidateState::Approved,
        mapping: None,
        observed_at: 220,
    };

    for observation in [
        ExternalItemObservation::Pending,
        ExternalItemObservation::PendingEdited {
            observed_title: Some("approved corrected title".to_owned()),
            observed_normalized_time: None,
        },
        ExternalItemObservation::approved_by_move("phase4-approved-real", "source-main"),
        ExternalItemObservation::approved_by_copy("phase4-approved-copy", "source-main"),
    ] {
        let plan = reconcile_candidate(&approved, &observation)?;

        assert!(plan.actions.is_empty(), "approved mutation suppression");
        assert_eq!(plan.suppressions, [Suppression::ApprovedMutationSuppressed]);
    }

    println!("PASS named assertion: approved mutation suppression");
    Ok(())
}

#[test]
fn rejects_malformed_phase4_observations_when_correction_or_approval_ids_are_invalid(
) -> Result<(), TestError> {
    let pending_err = reconcile_candidate(
        &lifecycle(CandidateState::Visible),
        &ExternalItemObservation::PendingEdited {
            observed_title: None,
            observed_normalized_time: None,
        },
    )
    .err()
    .ok_or(TestError::MissingExpectedError)?;
    assert_invalid_observation(&pending_err, "pending_edit")?;

    let move_err = reconcile_candidate(
        &lifecycle(CandidateState::Visible),
        &ExternalItemObservation::approved_by_move(" ", "source-main"),
    )
    .err()
    .ok_or(TestError::MissingExpectedError)?;
    assert_invalid_observation(&move_err, "external_object_id")?;

    let copy_err = reconcile_candidate(
        &lifecycle(CandidateState::Visible),
        &ExternalItemObservation::approved_by_copy("phase4-real-copy", ""),
    )
    .err()
    .ok_or(TestError::MissingExpectedError)?;
    assert_invalid_observation(&copy_err, "external_source_id")?;

    println!("PASS malformed_input assertions: invalid pending edit and blank approval IDs");
    Ok(())
}
