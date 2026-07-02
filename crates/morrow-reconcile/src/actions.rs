use morrow_storage::{CandidateId, CandidateState, ExternalSource};

/// A durable candidate lifecycle reason stored in audit rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleReason {
    /// The proposed external object was moved out of the Morrow proposal surface.
    ApprovedByMove,
    /// A matching real object was copied out and the proposed copy should be cleaned up.
    ApprovedByCopyCleanup,
    /// The proposed external object was deleted by the user.
    RejectedByDelete,
    /// A mapped external object disappeared without enough evidence of rejection.
    UnknownDisappearance,
    /// A visible proposed reminder was completed in the proposed list.
    ProposedReminderCompletedResolved,
    /// An approved real item reached its completed closure.
    CompletedClosure,
    /// A restart found the mapped external object and made the candidate visible.
    PartialWriteRecovered,
    /// External proposal creation failed before a durable mapping existed.
    ExternalCreationFailed,
    /// An update, reschedule, or cancellation proposal is informational only.
    ManualChangeProposalOnly,
    /// An older pending Morrow candidate was replaced by a newer same-anchor candidate.
    CandidateSuperseded,
    /// A reschedule proposal is manual-only in Phase 3.
    CandidateRescheduled,
    /// A cancellation proposal is manual-only in Phase 3.
    CandidateCancelled,
    /// A queued candidate is starting external proposal creation.
    CreatingExternalProposal,
}

impl LifecycleReason {
    /// Returns the stable storage/audit reason.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ApprovedByMove => "approved_by_move",
            Self::ApprovedByCopyCleanup => "approved_by_copy_cleanup",
            Self::RejectedByDelete => "rejected_by_delete",
            Self::UnknownDisappearance => "unknown_disappearance",
            Self::ProposedReminderCompletedResolved => "proposed_reminder_completed_resolved",
            Self::CompletedClosure => "completed_closure",
            Self::PartialWriteRecovered => "partial_write_recovered",
            Self::ExternalCreationFailed => "external_creation_failed",
            Self::ManualChangeProposalOnly => "manual_change_proposal_only",
            Self::CandidateSuperseded => "candidate_superseded",
            Self::CandidateRescheduled => "candidate_rescheduled",
            Self::CandidateCancelled => "candidate_cancelled",
            Self::CreatingExternalProposal => "creating_external_proposal",
        }
    }
}

/// A typed suppression emitted when reconciliation intentionally avoids mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suppression {
    /// The pending proposed item was edited by the user and must not be overwritten.
    EditedPendingNoOverwrite,
    /// Approved real objects are observation-only in the MVP.
    ApprovedMutationSuppressed,
    /// Update/reschedule/cancel proposals are manual/informational in the MVP.
    ManualChangeProposalOnly,
    /// Terminal candidates are closed to automatic lifecycle mutation.
    ClosedCandidate,
}

/// Field-level edit evidence for a pending proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingEditFeedback {
    /// The pending title was edited.
    pub title_edited: bool,
    /// The pending normalized time was edited.
    pub time_edited: bool,
    /// The observation timestamp used for feedback rows.
    pub observed_at: i64,
}

/// A mapping update to the observed approved external object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalMappingUpdate {
    /// The storage candidate ID owning the mapping.
    pub candidate_id: CandidateId,
    /// The external store family.
    pub source: ExternalSource,
    /// The durable external object ID.
    pub external_object_id: String,
    /// The external source/list/calendar ID.
    pub external_source_id: String,
    /// The observation timestamp used for the mapping.
    pub mapped_at: i64,
}

/// A lifecycle action produced by reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleAction {
    /// Start external proposal creation at the adapter boundary.
    CreateExternalProposal,
    /// Move a candidate to another storage state with an audit reason.
    Transition {
        /// The target candidate state.
        to: CandidateState,
        /// The stable audit reason.
        reason: LifecycleReason,
        /// The observation timestamp.
        observed_at: i64,
    },
    /// Update storage to point at the approved real external object.
    UpsertExternalMapping(ExternalMappingUpdate),
    /// Delete the leftover proposed copy from the fake/native proposed surface.
    CleanupProposedExternal {
        /// The old proposed external object ID to remove.
        external_object_id: String,
        /// The external store family.
        source: ExternalSource,
    },
}

/// The complete deterministic reconciliation result for one candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationPlan {
    /// Candidate being reconciled.
    pub candidate_id: CandidateId,
    /// Ordered lifecycle actions.
    pub actions: Vec<LifecycleAction>,
    /// Typed suppressions explaining intentionally skipped mutations.
    pub suppressions: Vec<Suppression>,
    /// Field-level feedback for a pending edit suppression.
    pub pending_edit_feedback: Option<PendingEditFeedback>,
}

impl ReconciliationPlan {
    /// Creates an empty plan for a candidate.
    pub const fn new(candidate_id: CandidateId) -> Self {
        Self {
            candidate_id,
            actions: Vec::new(),
            suppressions: Vec::new(),
            pending_edit_feedback: None,
        }
    }

    pub(crate) fn push_transition(
        &mut self,
        to: CandidateState,
        reason: LifecycleReason,
        observed_at: i64,
    ) {
        self.actions.push(LifecycleAction::Transition {
            to,
            reason,
            observed_at,
        });
    }

    pub(crate) fn set_pending_edit_feedback(&mut self, feedback: PendingEditFeedback) {
        self.pending_edit_feedback = Some(feedback);
    }
}
