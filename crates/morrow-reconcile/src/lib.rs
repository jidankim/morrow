//! Candidate lifecycle reconciliation for Morrow proposals.

mod actions;
mod apply;
mod decision;
mod error;
mod feedback;
pub mod feedback_eval;
mod observations;

pub use actions::{
    ExternalMappingUpdate, LifecycleAction, LifecycleReason, PendingEditFeedback,
    ReconciliationPlan, Suppression,
};
pub use apply::apply_reconciliation;
pub use decision::{reconcile_candidate, CandidateLifecycle};
pub use error::ReconcileError;
pub use observations::{DisappearanceEvidence, ExternalItemObservation};
