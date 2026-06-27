//! Candidate lifecycle reconciliation for Morrow proposals.

mod actions;
mod apply;
mod decision;
mod error;
mod observations;

pub use actions::{
    ExternalMappingUpdate, LifecycleAction, LifecycleReason, ReconciliationPlan, Suppression,
};
pub use apply::apply_reconciliation;
pub use decision::{reconcile_candidate, CandidateLifecycle};
pub use error::ReconcileError;
pub use observations::{DisappearanceEvidence, ExternalItemObservation};
