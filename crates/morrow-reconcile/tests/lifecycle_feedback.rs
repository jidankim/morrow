//! Reconciliation lifecycle feedback regression tests.

use morrow_reconcile::{
    apply_reconciliation, reconcile_candidate, CandidateLifecycle, DisappearanceEvidence,
    ExternalItemObservation, LifecycleReason,
};
use morrow_storage::{CandidateId, CandidateKind, CandidateState, ExternalSource, Store};
use std::process::Command;

/// Shared lifecycle test fixtures.
pub mod support;
use support::{db_path, mapping, mapping_with_source, visible_candidate, TestError};

#[test]
fn lifecycle_feedback_labels_records_observed_outcomes_once() -> Result<(), TestError> {
    let db_path = db_path("lifecycle_feedback_labels");
    let store = Store::open(&db_path)?;

    let moved = visible_candidate(&store, CandidateKind::CalendarEvent, "feedback-move")?;
    let moved_plan = reconcile_candidate(
        &CandidateLifecycle {
            candidate_id: moved.clone(),
            kind: CandidateKind::CalendarEvent,
            state: CandidateState::Visible,
            mapping: Some(mapping(&moved, "proposed-feedback-move")),
            observed_at: 120,
        },
        &ExternalItemObservation::approved_by_move("real-feedback-move", "source-main"),
    )?;
    apply_reconciliation(&store, &moved_plan)?;
    apply_reconciliation(&store, &moved_plan)?;

    let copied = visible_candidate(&store, CandidateKind::CalendarEvent, "feedback-copy")?;
    let copied_plan = reconcile_candidate(
        &CandidateLifecycle {
            candidate_id: copied.clone(),
            kind: CandidateKind::CalendarEvent,
            state: CandidateState::Visible,
            mapping: Some(mapping(&copied, "proposed-feedback-copy")),
            observed_at: 130,
        },
        &ExternalItemObservation::approved_by_copy("real-feedback-copy", "source-main"),
    )?;
    apply_reconciliation(&store, &copied_plan)?;
    apply_reconciliation(&store, &copied_plan)?;

    let deleted = visible_candidate(&store, CandidateKind::CalendarEvent, "feedback-delete")?;
    let deleted_plan = reconcile_candidate(
        &CandidateLifecycle {
            candidate_id: deleted.clone(),
            kind: CandidateKind::CalendarEvent,
            state: CandidateState::Visible,
            mapping: Some(mapping(&deleted, "proposed-feedback-delete")),
            observed_at: 140,
        },
        &ExternalItemObservation::DeletedFromProposed,
    )?;
    apply_reconciliation(&store, &deleted_plan)?;
    apply_reconciliation(&store, &deleted_plan)?;

    let completed = visible_candidate(&store, CandidateKind::TaskReminder, "feedback-complete")?;
    let completed_plan = reconcile_candidate(
        &CandidateLifecycle {
            candidate_id: completed.clone(),
            kind: CandidateKind::TaskReminder,
            state: CandidateState::Visible,
            mapping: Some(mapping_with_source(
                &completed,
                ExternalSource::Reminders,
                "proposed-feedback-complete",
            )),
            observed_at: 150,
        },
        &ExternalItemObservation::Completed,
    )?;
    apply_reconciliation(&store, &completed_plan)?;
    apply_reconciliation(&store, &completed_plan)?;

    let edited = visible_candidate(&store, CandidateKind::TaskReminder, "feedback-edit")?;
    let edited_plan = reconcile_candidate(
        &CandidateLifecycle {
            candidate_id: edited.clone(),
            kind: CandidateKind::TaskReminder,
            state: CandidateState::Visible,
            mapping: Some(mapping_with_source(
                &edited,
                ExternalSource::Reminders,
                "proposed-feedback-edit",
            )),
            observed_at: 160,
        },
        &ExternalItemObservation::PendingEdited {
            observed_title: Some("user edited title".to_owned()),
            observed_normalized_time: Some("2026-07-01T11:30:00Z".to_owned()),
        },
    )?;
    apply_reconciliation(&store, &edited_plan)?;
    apply_reconciliation(&store, &edited_plan)?;

    let failed = visible_candidate(&store, CandidateKind::CalendarEvent, "feedback-failed")?;
    let failed_plan = reconcile_candidate(
        &CandidateLifecycle {
            candidate_id: failed.clone(),
            kind: CandidateKind::CalendarEvent,
            state: CandidateState::Visible,
            mapping: Some(mapping(&failed, "proposed-feedback-failed")),
            observed_at: 170,
        },
        &ExternalItemObservation::CreationFailed,
    )?;
    apply_reconciliation(&store, &failed_plan)?;
    apply_reconciliation(&store, &failed_plan)?;

    assert_feedback_event_count(&db_path, "approved_by_move", 1)?;
    assert_audit_reason_count(&store, &moved, LifecycleReason::ApprovedByMove, 1)?;
    assert_feedback_label_count(&db_path, "proposal_outcome", "accepted", 2)?;
    assert_feedback_event_count(&db_path, "approved_by_copy", 1)?;
    assert_audit_reason_count(&store, &copied, LifecycleReason::ApprovedByCopyCleanup, 1)?;
    assert_feedback_event_count(&db_path, "rejected_by_delete", 1)?;
    assert_audit_reason_count(&store, &deleted, LifecycleReason::RejectedByDelete, 1)?;
    assert_feedback_label_count(&db_path, "proposal_outcome", "rejected_observed", 2)?;
    assert_feedback_event_count(&db_path, "proposed_reminder_completed_resolved", 1)?;
    assert_audit_reason_count(
        &store,
        &completed,
        LifecycleReason::ProposedReminderCompletedResolved,
        1,
    )?;
    assert_feedback_event_count(&db_path, "pending_edited", 1)?;
    assert_feedback_label_count(&db_path, "proposal_outcome", "pending_edited", 1)?;
    assert_feedback_label_count(&db_path, "field_quality", "title_edited", 1)?;
    assert_feedback_label_count(&db_path, "field_quality", "time_edited", 1)?;
    assert_feedback_event_count(&db_path, "external_creation_failed", 1)?;
    assert_audit_reason_count(&store, &failed, LifecycleReason::ExternalCreationFailed, 1)?;
    assert_feedback_label_count(&db_path, "system_outcome", "failed_external_creation", 1)?;

    Ok(())
}

#[test]
fn lifecycle_feedback_unknown_disappearance_label() -> Result<(), TestError> {
    let db_path = db_path("lifecycle_feedback_unknown_disappearance_label");
    let store = Store::open(&db_path)?;
    let unknown = visible_candidate(&store, CandidateKind::CalendarEvent, "feedback-unknown")?;
    let unknown_plan = reconcile_candidate(
        &CandidateLifecycle {
            candidate_id: unknown.clone(),
            kind: CandidateKind::CalendarEvent,
            state: CandidateState::Visible,
            mapping: Some(mapping(&unknown, "proposed-feedback-unknown")),
            observed_at: 180,
        },
        &ExternalItemObservation::Disappeared {
            evidence: DisappearanceEvidence::StoreResetOrPermissionGap,
        },
    )?;

    apply_reconciliation(&store, &unknown_plan)?;
    apply_reconciliation(&store, &unknown_plan)?;

    assert_feedback_event_count(&db_path, "unknown_disappearance", 1)?;
    assert_audit_reason_count(&store, &unknown, LifecycleReason::UnknownDisappearance, 1)?;
    assert_feedback_label_count(&db_path, "proposal_outcome", "unknown", 1)?;
    assert_feedback_label_count(&db_path, "proposal_outcome", "accepted", 0)?;
    assert_feedback_label_count(&db_path, "proposal_outcome", "false_positive", 0)?;
    Ok(())
}

fn assert_audit_reason_count(
    store: &Store,
    candidate_id: &CandidateId,
    reason: LifecycleReason,
    expected: usize,
) -> Result<(), TestError> {
    let actual = store
        .audit_entries(candidate_id)?
        .iter()
        .filter(|entry| entry.reason == reason.as_str())
        .count();
    if actual != expected {
        return Err(TestError::Command(format!(
            "expected audit reason {} count {expected}, got {actual}",
            reason.as_str()
        )));
    }
    Ok(())
}

fn assert_feedback_event_count(
    db_path: &std::path::Path,
    event_type: &str,
    expected: i64,
) -> Result<(), TestError> {
    let sql = format!("SELECT COUNT(*) FROM feedback_events WHERE event_type = '{event_type}';");
    let actual = sqlite_count(db_path, &sql)?;
    if actual != expected {
        return Err(TestError::Command(format!(
            "expected feedback event {event_type} count {expected}, got {actual}"
        )));
    }
    Ok(())
}

fn assert_feedback_label_count(
    db_path: &std::path::Path,
    label_type: &str,
    label_value: &str,
    expected: i64,
) -> Result<(), TestError> {
    let sql = format!(
        "SELECT COUNT(*) FROM labels WHERE label_type = '{label_type}' AND label_value = '{label_value}';"
    );
    let actual = sqlite_count(db_path, &sql)?;
    if actual != expected {
        return Err(TestError::Command(format!(
            "expected feedback label {label_type}={label_value} count {expected}, got {actual}"
        )));
    }
    Ok(())
}

fn sqlite_count(db_path: &std::path::Path, sql: &str) -> Result<i64, TestError> {
    let output = Command::new("sqlite3")
        .arg(db_path)
        .arg(sql)
        .output()
        .map_err(|err| TestError::Command(err.to_string()))?;
    if !output.status.success() {
        return Err(TestError::Command(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<i64>()
        .map_err(|err| TestError::Command(err.to_string()))
}
