#![allow(clippy::redundant_pub_crate)]

use std::io::Error;

use morrow_reconcile::{
    apply_reconciliation, reconcile_candidate, DisappearanceEvidence, ExternalItemObservation,
    LifecycleReason, Suppression,
};
use morrow_storage::{CandidateKind, CandidateState, ExternalSource, Store};

use crate::summary::SmokeCounts;

#[path = "fixtures.rs"]
mod fixtures;

use fixtures::{count_reason, db_path, lifecycle, mapping, prepare_candidate};

pub(crate) fn run_lifecycle_suite() -> Result<SmokeCounts, Box<dyn std::error::Error>> {
    let store = Store::open(&db_path())?;
    Ok(SmokeCounts {
        approved_by_move: run_audited_case(
            &store,
            "approved-move",
            CandidateKind::CalendarEvent,
            CandidateState::Visible,
            &ExternalItemObservation::approved_by_move("real-move", "source-main"),
            LifecycleReason::ApprovedByMove,
        )?,
        approved_by_copy_cleanup: run_audited_case(
            &store,
            "approved-copy",
            CandidateKind::CalendarEvent,
            CandidateState::Visible,
            &ExternalItemObservation::approved_by_copy("real-copy", "source-main"),
            LifecycleReason::ApprovedByCopyCleanup,
        )?,
        rejected_by_delete: run_audited_case(
            &store,
            "rejected-delete",
            CandidateKind::CalendarEvent,
            CandidateState::Visible,
            &ExternalItemObservation::DeletedFromProposed,
            LifecycleReason::RejectedByDelete,
        )?,
        unknown_disappearance: run_audited_case(
            &store,
            "unknown-disappearance",
            CandidateKind::CalendarEvent,
            CandidateState::Visible,
            &ExternalItemObservation::Disappeared {
                evidence: DisappearanceEvidence::StoreResetOrPermissionGap,
            },
            LifecycleReason::UnknownDisappearance,
        )?,
        edited_pending_no_overwrite: run_suppression_case(&store)?,
        completed_closure: run_completed_case(&store)?,
        partial_write_recovered: run_partial_write_case(&store)?,
    })
}

fn run_audited_case(
    store: &Store,
    suffix: &str,
    kind: CandidateKind,
    state: CandidateState,
    observation: &ExternalItemObservation,
    reason: LifecycleReason,
) -> Result<usize, Box<dyn std::error::Error>> {
    let candidate_id = prepare_candidate(store, kind, suffix, state)?;
    let plan = reconcile_candidate(
        &lifecycle(&candidate_id, kind, state, suffix, 20),
        observation,
    )?;
    apply_reconciliation(store, &plan)?;
    count_reason(store, &candidate_id, reason)
}

fn run_suppression_case(store: &Store) -> Result<usize, Box<dyn std::error::Error>> {
    let candidate_id = prepare_candidate(
        store,
        CandidateKind::TaskReminder,
        "edited-pending",
        CandidateState::Visible,
    )?;
    let plan = reconcile_candidate(
        &lifecycle(
            &candidate_id,
            CandidateKind::TaskReminder,
            CandidateState::Visible,
            "edited-pending",
            30,
        ),
        &ExternalItemObservation::PendingEdited {
            observed_title: "user edited".to_owned(),
        },
    )?;
    if plan
        .suppressions
        .contains(&Suppression::EditedPendingNoOverwrite)
    {
        Ok(1)
    } else {
        Err(Box::new(Error::other("edited pending suppression missing")))
    }
}

fn run_completed_case(store: &Store) -> Result<usize, Box<dyn std::error::Error>> {
    let candidate_id = prepare_candidate(
        store,
        CandidateKind::TaskReminder,
        "completed",
        CandidateState::Approved,
    )?;
    let plan = reconcile_candidate(
        &lifecycle(
            &candidate_id,
            CandidateKind::TaskReminder,
            CandidateState::Approved,
            "completed",
            40,
        ),
        &ExternalItemObservation::Completed,
    )?;
    apply_reconciliation(store, &plan)?;
    count_reason(store, &candidate_id, LifecycleReason::CompletedClosure)
}

fn run_partial_write_case(store: &Store) -> Result<usize, Box<dyn std::error::Error>> {
    let candidate_id = prepare_candidate(
        store,
        CandidateKind::CalendarEvent,
        "partial",
        CandidateState::CreatingExternal,
    )?;
    let known_mapping = mapping(
        &candidate_id,
        ExternalSource::Calendar,
        "partial-external",
        1,
    );
    store.upsert_external_mapping(known_mapping.clone())?;
    let mut input = lifecycle(
        &candidate_id,
        CandidateKind::CalendarEvent,
        CandidateState::CreatingExternal,
        "partial",
        50,
    );
    input.mapping = Some(known_mapping);
    let plan = reconcile_candidate(&input, &ExternalItemObservation::Pending)?;
    apply_reconciliation(store, &plan)?;
    count_reason(store, &candidate_id, LifecycleReason::PartialWriteRecovered)
}
