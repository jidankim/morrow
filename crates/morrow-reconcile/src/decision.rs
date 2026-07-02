use morrow_storage::{
    validate_normalized_time, CandidateId, CandidateKind, CandidateState, ExternalObjectMapping,
    StorageError,
};

#[path = "kind.rs"]
mod kind;
#[path = "state_decisions.rs"]
mod state_decisions;

use crate::{
    ExternalItemObservation, LifecycleReason, ReconcileError, ReconciliationPlan, Suppression,
};
use kind::{is_manual_change, source_for_kind};
use state_decisions::{
    reconcile_approved, reconcile_creating_external, reconcile_manual_change, reconcile_queued,
    reconcile_visible,
};

/// Candidate storage state and known external mapping at reconciliation time.
#[derive(Debug, Clone)]
pub struct CandidateLifecycle {
    /// Candidate being reconciled.
    pub candidate_id: CandidateId,
    /// Candidate kind.
    pub kind: CandidateKind,
    /// Current durable candidate state.
    pub state: CandidateState,
    /// Known external object mapping, if one exists.
    pub mapping: Option<ExternalObjectMapping>,
    /// Observation timestamp.
    pub observed_at: i64,
}

/// Reconciles one candidate and one external observation into deterministic actions.
pub fn reconcile_candidate(
    candidate: &CandidateLifecycle,
    observation: &ExternalItemObservation,
) -> Result<ReconciliationPlan, ReconcileError> {
    validate_mapping_owner(candidate)?;
    validate_observation(observation)?;

    let mut plan = ReconciliationPlan::new(candidate.candidate_id.clone());
    if is_manual_change(candidate.kind) {
        reconcile_manual_change(candidate, &mut plan);
        return Ok(plan);
    }

    match candidate.state {
        CandidateState::Queued => reconcile_queued(candidate, observation, &mut plan),
        CandidateState::CreatingExternal => {
            reconcile_creating_external(candidate, observation, &mut plan);
        }
        CandidateState::Visible => reconcile_visible(candidate, observation, &mut plan),
        CandidateState::Approved => reconcile_approved(candidate, observation, &mut plan),
        CandidateState::Completed
        | CandidateState::Rejected
        | CandidateState::Expired
        | CandidateState::Suppressed
        | CandidateState::Unknown
        | CandidateState::Failed => plan.suppressions.push(Suppression::ClosedCandidate),
    }

    Ok(plan)
}

/// Builds deterministic supersede plans for candidates returned by a same-anchor lookup.
pub fn reconcile_same_anchor_supersede_candidates(
    newer_candidate: &CandidateLifecycle,
    older_same_anchor_candidates: &[CandidateLifecycle],
) -> Vec<ReconciliationPlan> {
    older_same_anchor_candidates
        .iter()
        .filter(|candidate| candidate.candidate_id != newer_candidate.candidate_id)
        .filter(|candidate| {
            source_for_kind(candidate.kind) == source_for_kind(newer_candidate.kind)
        })
        .map(|candidate| {
            let mut plan = ReconciliationPlan::new(candidate.candidate_id.clone());
            match candidate.state {
                CandidateState::Queued
                | CandidateState::CreatingExternal
                | CandidateState::Visible => {
                    plan.push_transition(
                        CandidateState::Suppressed,
                        LifecycleReason::CandidateSuperseded,
                        newer_candidate.observed_at,
                    );
                }
                CandidateState::Approved => {
                    plan.suppressions
                        .push(Suppression::ApprovedMutationSuppressed);
                }
                CandidateState::Completed
                | CandidateState::Rejected
                | CandidateState::Expired
                | CandidateState::Suppressed
                | CandidateState::Unknown
                | CandidateState::Failed => plan.suppressions.push(Suppression::ClosedCandidate),
            }
            plan
        })
        .collect()
}

fn validate_mapping_owner(candidate: &CandidateLifecycle) -> Result<(), ReconcileError> {
    let Some(mapping) = &candidate.mapping else {
        return Ok(());
    };
    if mapping.candidate_id == candidate.candidate_id {
        Ok(())
    } else {
        Err(ReconcileError::InvalidMappingCandidate {
            expected: candidate.candidate_id.to_string(),
            actual: mapping.candidate_id.to_string(),
        })
    }
}

fn validate_observation(observation: &ExternalItemObservation) -> Result<(), ReconcileError> {
    match observation {
        ExternalItemObservation::ApprovedByMove {
            external_object_id,
            external_source_id,
        }
        | ExternalItemObservation::ApprovedByCopy {
            external_object_id,
            external_source_id,
        } => {
            validate_non_empty("external_object_id", external_object_id)?;
            validate_non_empty("external_source_id", external_source_id)
        }
        ExternalItemObservation::PendingEdited {
            observed_title,
            observed_normalized_time,
        } => {
            validate_pending_edit_present(observed_title, observed_normalized_time)?;
            if let Some(title) = observed_title {
                validate_non_empty("observed_title", title)?;
            }
            if let Some(normalized_time) = observed_normalized_time {
                validate_pending_normalized_time(normalized_time)?;
            }
            Ok(())
        }
        ExternalItemObservation::Pending
        | ExternalItemObservation::DeletedFromProposed
        | ExternalItemObservation::Disappeared { .. }
        | ExternalItemObservation::Completed
        | ExternalItemObservation::CreationFailed => Ok(()),
    }
}

fn validate_pending_edit_present(
    observed_title: &Option<String>,
    observed_normalized_time: &Option<String>,
) -> Result<(), ReconcileError> {
    if observed_title.is_none() && observed_normalized_time.is_none() {
        Err(ReconcileError::InvalidObservation {
            field: "pending_edit",
            reason: "must include an observed title or normalized time".to_owned(),
        })
    } else {
        Ok(())
    }
}

fn validate_pending_normalized_time(value: &str) -> Result<(), ReconcileError> {
    validate_normalized_time(value).map_err(|err| match err {
        StorageError::InvalidInput { reason, .. } => ReconcileError::InvalidObservation {
            field: "observed_normalized_time",
            reason,
        },
        other => ReconcileError::Storage(other),
    })
}

fn validate_non_empty(field: &'static str, value: &str) -> Result<(), ReconcileError> {
    if value.trim().is_empty() {
        Err(ReconcileError::InvalidObservation {
            field,
            reason: "must not be empty".to_owned(),
        })
    } else {
        Ok(())
    }
}
