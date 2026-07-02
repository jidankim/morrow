use morrow_storage::{CandidateId, CandidateKind, CandidateState, ExternalObjectMapping, Store};

use super::decision::ReplayStoreDecision;
use super::{
    mapping_finalization_decision, storage_error, ProposalReplayDelta, ProposalReplaySummary,
    ScanSelectedChatsError, MAPPED_AT,
};

const CALENDAR_COMMIT_IDEMPOTENCY: &str = "calendar_commit_idempotency";

pub(super) fn finalize_existing_mapping(
    store: &Store,
    kind: CandidateKind,
    mapping: &ExternalObjectMapping,
    summary: &mut ProposalReplaySummary,
) -> Result<(), ScanSelectedChatsError> {
    let state = store
        .candidate_state(&mapping.candidate_id)
        .map_err(storage_error)?;
    match (kind, state) {
        (CandidateKind::CalendarEvent, CandidateState::Visible) => {
            store
                .record_external_replay_recovery_pending(
                    &mapping.candidate_id,
                    CALENDAR_COMMIT_IDEMPOTENCY,
                    MAPPED_AT,
                )
                .map_err(storage_error)?;
            summary.calendar_commit_idempotency += 1;
            Ok(())
        }
        (
            CandidateKind::TaskReminder
            | CandidateKind::EventUpdate
            | CandidateKind::EventReschedule
            | CandidateKind::EventCancellation
            | CandidateKind::ReminderUpdate
            | CandidateKind::ReminderReschedule
            | CandidateKind::ReminderCancellation,
            CandidateState::Visible,
        ) => Ok(()),
        (
            CandidateKind::CalendarEvent
            | CandidateKind::TaskReminder
            | CandidateKind::EventUpdate
            | CandidateKind::EventReschedule
            | CandidateKind::EventCancellation
            | CandidateKind::ReminderUpdate
            | CandidateKind::ReminderReschedule
            | CandidateKind::ReminderCancellation,
            CandidateState::Queued
            | CandidateState::CreatingExternal
            | CandidateState::Approved
            | CandidateState::Completed
            | CandidateState::Rejected
            | CandidateState::Expired
            | CandidateState::Suppressed
            | CandidateState::Unknown
            | CandidateState::Failed,
        ) => {
            let store_decision =
                mapping_finalization_decision(false, transition_mapping_visible(store, mapping));
            summary.apply(apply_store_decision(
                store,
                &mapping.candidate_id,
                store_decision,
            )?);
            Ok(())
        }
    }
}

pub(super) fn transition_mapping_visible(store: &Store, mapping: &ExternalObjectMapping) -> bool {
    store
        .upsert_external_mapping(mapping.clone())
        .and_then(|()| {
            store.transition_candidate(
                &mapping.candidate_id,
                CandidateState::Visible,
                "external_proposal_created",
                MAPPED_AT,
            )
        })
        .is_ok()
}

pub(super) fn apply_store_decision(
    store: &Store,
    candidate_id: &CandidateId,
    decision: ReplayStoreDecision,
) -> Result<ProposalReplayDelta, ScanSelectedChatsError> {
    match decision {
        ReplayStoreDecision::TransitionVisible { summary_delta } => Ok(summary_delta),
        ReplayStoreDecision::MarkRecoveryPending { summary_delta } => {
            store
                .record_external_replay_recovery_pending(
                    candidate_id,
                    "external_proposal_recovery_pending",
                    MAPPED_AT,
                )
                .map_err(storage_error)?;
            Ok(summary_delta)
        }
        ReplayStoreDecision::MarkFailed {
            reason,
            summary_delta,
        } => {
            store
                .transition_candidate(candidate_id, CandidateState::Failed, &reason, MAPPED_AT)
                .map_err(storage_error)?;
            Ok(summary_delta)
        }
    }
}
