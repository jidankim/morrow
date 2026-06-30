use std::path::Path;

use morrow_reconcile::{
    apply_reconciliation, reconcile_candidate, CandidateLifecycle, ExternalItemObservation,
    LifecycleAction,
};
use morrow_storage::{
    CandidateDraft, CandidateId, CandidateKind, CandidateState, DeleteAllConfirmation,
    ExternalObjectMapping, ExternalSource, Store,
};

use crate::feedback::record_candidate_feedback;

/// Lifecycle scenario aggregate counts.
#[derive(Debug)]
pub struct LifecycleSummary {
    /// Number of visible proposal scenarios.
    pub visible_proposals: u64,
    /// Number of approval scenarios.
    pub approvals: u64,
    /// Number of deletion scenarios.
    pub deletions: u64,
    /// Synthetic latency for metrics.
    pub latency_seconds: u64,
}

/// Delete All scenario summary.
#[derive(Debug)]
pub struct DeleteSummary {
    /// Whether the storage database was deleted.
    pub database_deleted: bool,
    /// Pre-delete and sidecar readbacks.
    pub readbacks: DeleteReadbacks,
}

/// Delete All readback checks.
#[derive(Debug)]
pub struct DeleteReadbacks {
    /// Approved candidate was readable before deletion.
    pub approved_candidate_read_before_delete: bool,
    /// Feedback/eval rows were readable before deletion.
    pub feedback_eval_case_read_before_delete: bool,
    /// Non-Morrow external sidecar was preserved.
    pub fake_approved_external_item_preserved: bool,
    /// Apple Messages sidecar was preserved.
    pub fake_apple_message_source_preserved: bool,
}

/// Runs the lifecycle approval, rejection, completion, and partial-write cases.
///
/// # Panics
///
/// Panics when the generated plans do not contain the expected observable action.
pub fn run_lifecycle_cases(store: &Store) -> Result<LifecycleSummary, Box<dyn std::error::Error>> {
    let moved = visible_candidate(store, CandidateKind::CalendarEvent, "move")?;
    let moved_plan = reconcile_candidate(
        &lifecycle(
            &moved,
            CandidateKind::CalendarEvent,
            CandidateState::Visible,
            "proposed-move",
        ),
        &ExternalItemObservation::approved_by_move("real-move", "calendar-main"),
    )?;
    apply_reconciliation(store, &moved_plan)?;

    let copied = visible_candidate(store, CandidateKind::CalendarEvent, "copy")?;
    let copied_plan = reconcile_candidate(
        &lifecycle(
            &copied,
            CandidateKind::CalendarEvent,
            CandidateState::Visible,
            "proposed-copy",
        ),
        &ExternalItemObservation::approved_by_copy("real-copy", "calendar-main"),
    )?;
    assert!(copied_plan.actions.iter().any(|action| matches!(
        action,
        LifecycleAction::CleanupProposedExternal { external_object_id, .. }
            if external_object_id == "proposed-copy"
    )));
    apply_reconciliation(store, &copied_plan)?;

    let deleted = visible_candidate(store, CandidateKind::CalendarEvent, "delete")?;
    let deleted_plan = reconcile_candidate(
        &lifecycle(
            &deleted,
            CandidateKind::CalendarEvent,
            CandidateState::Visible,
            "proposed-delete",
        ),
        &ExternalItemObservation::DeletedFromProposed,
    )?;
    apply_reconciliation(store, &deleted_plan)?;

    let completed = visible_candidate(store, CandidateKind::TaskReminder, "complete")?;
    store.transition_candidate(&completed, CandidateState::Approved, "setup_approved", 31)?;
    let completed_plan = reconcile_candidate(
        &lifecycle(
            &completed,
            CandidateKind::TaskReminder,
            CandidateState::Approved,
            "real-reminder",
        ),
        &ExternalItemObservation::Completed,
    )?;
    apply_reconciliation(store, &completed_plan)?;

    assert_eq!(store.candidate_state(&moved)?, CandidateState::Approved);
    assert_eq!(store.candidate_state(&copied)?, CandidateState::Approved);
    assert_eq!(store.candidate_state(&deleted)?, CandidateState::Rejected);
    assert_eq!(
        store.candidate_state(&completed)?,
        CandidateState::Completed
    );

    Ok(LifecycleSummary {
        visible_proposals: 4,
        approvals: 2,
        deletions: 1,
        latency_seconds: 12,
    })
}

/// Runs the Delete All storage boundary case.
pub fn run_delete_all_case(delete_db: &Path) -> Result<DeleteSummary, Box<dyn std::error::Error>> {
    let store = Store::open(delete_db)?;
    let delete_draft = draft(CandidateKind::CalendarEvent, "delete-all");
    let candidate_id = store.create_candidate(delete_draft.clone())?;
    record_candidate_feedback(&store, &delete_draft)?;
    store.transition_candidate(
        &candidate_id,
        CandidateState::CreatingExternal,
        "delete_all_setup_creating",
        2,
    )?;
    store.transition_candidate(
        &candidate_id,
        CandidateState::Visible,
        "delete_all_setup_visible",
        3,
    )?;
    store.transition_candidate(
        &candidate_id,
        CandidateState::Approved,
        "delete_all_setup_approved",
        4,
    )?;
    let approved_external_item = delete_db.with_extension("approved_external.txt");
    let apple_message_source = delete_db.with_extension("apple_messages.txt");
    std::fs::write(&approved_external_item, "approved-calendar-item:keep")?;
    std::fs::write(&apple_message_source, "apple-message-source:keep")?;
    let approved_candidate_read_before_delete =
        store.candidate_state(&candidate_id)? == CandidateState::Approved;
    let feedback_eval_case_read_before_delete = store.eval_cases()?.len() == 1;
    let receipt = store.delete_all(DeleteAllConfirmation::parse("DELETE MORROW DATA")?)?;
    let fake_approved_external_item_preserved =
        std::fs::read_to_string(&approved_external_item)? == "approved-calendar-item:keep";
    let fake_apple_message_source_preserved =
        std::fs::read_to_string(&apple_message_source)? == "apple-message-source:keep";
    Ok(DeleteSummary {
        database_deleted: receipt.database_deleted && !delete_db.exists(),
        readbacks: DeleteReadbacks {
            approved_candidate_read_before_delete,
            feedback_eval_case_read_before_delete,
            fake_approved_external_item_preserved: !receipt.approved_external_items_deleted
                && fake_approved_external_item_preserved,
            fake_apple_message_source_preserved,
        },
    })
}

fn visible_candidate(
    store: &Store,
    kind: CandidateKind,
    suffix: &str,
) -> Result<CandidateId, Box<dyn std::error::Error>> {
    let candidate_id = store.create_candidate(draft(kind, suffix))?;
    store.transition_candidate(
        &candidate_id,
        CandidateState::CreatingExternal,
        "setup_creating",
        11,
    )?;
    store.upsert_external_mapping(mapping(&candidate_id, kind, &format!("proposed-{suffix}")))?;
    store.transition_candidate(&candidate_id, CandidateState::Visible, "setup_visible", 12)?;
    Ok(candidate_id)
}

fn lifecycle(
    candidate_id: &CandidateId,
    kind: CandidateKind,
    state: CandidateState,
    external_object_id: &str,
) -> CandidateLifecycle {
    CandidateLifecycle {
        candidate_id: candidate_id.clone(),
        kind,
        state,
        mapping: Some(mapping(candidate_id, kind, external_object_id)),
        observed_at: 40,
    }
}

fn mapping(
    candidate_id: &CandidateId,
    kind: CandidateKind,
    external_object_id: &str,
) -> ExternalObjectMapping {
    ExternalObjectMapping {
        candidate_id: candidate_id.clone(),
        source: match kind {
            CandidateKind::CalendarEvent
            | CandidateKind::EventUpdate
            | CandidateKind::EventReschedule
            | CandidateKind::EventCancellation => ExternalSource::Calendar,
            CandidateKind::TaskReminder
            | CandidateKind::ReminderUpdate
            | CandidateKind::ReminderReschedule
            | CandidateKind::ReminderCancellation => ExternalSource::Reminders,
        },
        external_object_id: external_object_id.to_owned(),
        external_source_id: "source-main".to_owned(),
        mapped_at: 12,
    }
}

fn draft(kind: CandidateKind, suffix: &str) -> CandidateDraft {
    CandidateDraft {
        kind,
        chat_guid: format!("chat-{suffix}"),
        anchor_message_guid: format!("message-{suffix}"),
        title: format!("Candidate {suffix}"),
        confidence_millis: 850,
        normalized_time: "2026-07-05T10:00:00[Asia/Seoul]".to_owned(),
        evidence_excerpt: "Short synthetic excerpt".to_owned(),
        observed_at: 10,
    }
}
