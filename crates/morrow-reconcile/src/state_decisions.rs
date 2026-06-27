use morrow_storage::{CandidateKind, CandidateState};

use super::kind::source_for_kind;
use crate::{
    CandidateLifecycle, DisappearanceEvidence, ExternalItemObservation, ExternalMappingUpdate,
    LifecycleAction, LifecycleReason, ReconciliationPlan, Suppression,
};

pub(super) fn reconcile_queued(
    candidate: &CandidateLifecycle,
    observation: &ExternalItemObservation,
    plan: &mut ReconciliationPlan,
) {
    match observation {
        ExternalItemObservation::CreationFailed => plan.push_transition(
            CandidateState::Failed,
            LifecycleReason::ExternalCreationFailed,
            candidate.observed_at,
        ),
        ExternalItemObservation::DeletedFromProposed
        | ExternalItemObservation::Disappeared {
            evidence: DisappearanceEvidence::UserDeletedProposed,
        } => plan.push_transition(
            CandidateState::Rejected,
            LifecycleReason::RejectedByDelete,
            candidate.observed_at,
        ),
        ExternalItemObservation::Pending
        | ExternalItemObservation::PendingEdited { .. }
        | ExternalItemObservation::ApprovedByMove { .. }
        | ExternalItemObservation::ApprovedByCopy { .. }
        | ExternalItemObservation::Disappeared { .. }
        | ExternalItemObservation::Completed => {
            plan.actions.push(LifecycleAction::CreateExternalProposal);
            plan.push_transition(
                CandidateState::CreatingExternal,
                LifecycleReason::CreatingExternalProposal,
                candidate.observed_at,
            );
        }
    }
}

pub(super) fn reconcile_creating_external(
    candidate: &CandidateLifecycle,
    observation: &ExternalItemObservation,
    plan: &mut ReconciliationPlan,
) {
    match observation {
        ExternalItemObservation::Pending
        | ExternalItemObservation::PendingEdited { .. }
        | ExternalItemObservation::ApprovedByMove { .. }
        | ExternalItemObservation::ApprovedByCopy { .. }
            if candidate.mapping.is_some() =>
        {
            plan.push_transition(
                CandidateState::Visible,
                LifecycleReason::PartialWriteRecovered,
                candidate.observed_at,
            );
        }
        ExternalItemObservation::CreationFailed
        | ExternalItemObservation::Disappeared { .. }
        | ExternalItemObservation::DeletedFromProposed
        | ExternalItemObservation::Completed
        | ExternalItemObservation::Pending
        | ExternalItemObservation::PendingEdited { .. }
        | ExternalItemObservation::ApprovedByMove { .. }
        | ExternalItemObservation::ApprovedByCopy { .. } => plan.push_transition(
            CandidateState::Failed,
            LifecycleReason::ExternalCreationFailed,
            candidate.observed_at,
        ),
    }
}

pub(super) fn reconcile_visible(
    candidate: &CandidateLifecycle,
    observation: &ExternalItemObservation,
    plan: &mut ReconciliationPlan,
) {
    match observation {
        ExternalItemObservation::Pending => {}
        ExternalItemObservation::PendingEdited { .. } => {
            plan.suppressions
                .push(Suppression::EditedPendingNoOverwrite);
        }
        ExternalItemObservation::ApprovedByMove {
            external_object_id,
            external_source_id,
        } => approve(
            candidate,
            plan,
            external_object_id.clone(),
            external_source_id.clone(),
            false,
        ),
        ExternalItemObservation::ApprovedByCopy {
            external_object_id,
            external_source_id,
        } => approve(
            candidate,
            plan,
            external_object_id.clone(),
            external_source_id.clone(),
            true,
        ),
        ExternalItemObservation::DeletedFromProposed => plan.push_transition(
            CandidateState::Rejected,
            LifecycleReason::RejectedByDelete,
            candidate.observed_at,
        ),
        ExternalItemObservation::Disappeared { evidence } => {
            reconcile_visible_disappearance(candidate, *evidence, plan);
        }
        ExternalItemObservation::Completed if candidate.kind == CandidateKind::TaskReminder => {
            plan.push_transition(
                CandidateState::Rejected,
                LifecycleReason::ProposedReminderCompletedResolved,
                candidate.observed_at,
            );
        }
        ExternalItemObservation::Completed | ExternalItemObservation::CreationFailed => {
            plan.push_transition(
                CandidateState::Failed,
                LifecycleReason::ExternalCreationFailed,
                candidate.observed_at,
            );
        }
    }
}

pub(super) fn reconcile_approved(
    candidate: &CandidateLifecycle,
    observation: &ExternalItemObservation,
    plan: &mut ReconciliationPlan,
) {
    match observation {
        ExternalItemObservation::Completed => plan.push_transition(
            CandidateState::Completed,
            LifecycleReason::CompletedClosure,
            candidate.observed_at,
        ),
        ExternalItemObservation::Disappeared { .. }
        | ExternalItemObservation::DeletedFromProposed
        | ExternalItemObservation::CreationFailed => plan.push_transition(
            CandidateState::Unknown,
            LifecycleReason::UnknownDisappearance,
            candidate.observed_at,
        ),
        ExternalItemObservation::Pending
        | ExternalItemObservation::PendingEdited { .. }
        | ExternalItemObservation::ApprovedByMove { .. }
        | ExternalItemObservation::ApprovedByCopy { .. } => {
            plan.suppressions
                .push(Suppression::ApprovedMutationSuppressed);
        }
    }
}

pub(super) fn reconcile_manual_change(
    candidate: &CandidateLifecycle,
    plan: &mut ReconciliationPlan,
) {
    plan.suppressions
        .push(Suppression::ManualChangeProposalOnly);
    match candidate.state {
        CandidateState::Queued | CandidateState::CreatingExternal | CandidateState::Visible => {
            plan.push_transition(
                CandidateState::Suppressed,
                LifecycleReason::ManualChangeProposalOnly,
                candidate.observed_at,
            );
        }
        CandidateState::Approved => plan
            .suppressions
            .push(Suppression::ApprovedMutationSuppressed),
        CandidateState::Completed
        | CandidateState::Rejected
        | CandidateState::Expired
        | CandidateState::Suppressed
        | CandidateState::Unknown
        | CandidateState::Failed => plan.suppressions.push(Suppression::ClosedCandidate),
    }
}

fn approve(
    candidate: &CandidateLifecycle,
    plan: &mut ReconciliationPlan,
    external_object_id: String,
    external_source_id: String,
    cleanup_copy: bool,
) {
    plan.actions.push(LifecycleAction::UpsertExternalMapping(
        ExternalMappingUpdate {
            candidate_id: candidate.candidate_id.clone(),
            source: source_for_kind(candidate.kind),
            external_object_id,
            external_source_id,
            mapped_at: candidate.observed_at,
        },
    ));
    if cleanup_copy {
        if let Some(mapping) = &candidate.mapping {
            plan.actions.push(LifecycleAction::CleanupProposedExternal {
                external_object_id: mapping.external_object_id.clone(),
                source: mapping.source,
            });
        }
    }
    plan.push_transition(
        CandidateState::Approved,
        if cleanup_copy {
            LifecycleReason::ApprovedByCopyCleanup
        } else {
            LifecycleReason::ApprovedByMove
        },
        candidate.observed_at,
    );
}

fn reconcile_visible_disappearance(
    candidate: &CandidateLifecycle,
    evidence: DisappearanceEvidence,
    plan: &mut ReconciliationPlan,
) {
    match evidence {
        DisappearanceEvidence::UserDeletedProposed => plan.push_transition(
            CandidateState::Rejected,
            LifecycleReason::RejectedByDelete,
            candidate.observed_at,
        ),
        DisappearanceEvidence::StoreResetOrPermissionGap
        | DisappearanceEvidence::NoMorrowMetadata => {
            plan.push_transition(
                CandidateState::Unknown,
                LifecycleReason::UnknownDisappearance,
                candidate.observed_at,
            );
        }
    }
}
