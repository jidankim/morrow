use morrow_storage::{ExternalObjectMapping, Store};

use crate::feedback::record_lifecycle_feedback;
use crate::{LifecycleAction, ReconcileError, ReconciliationPlan};

/// Applies storage-backed reconciliation actions and lets storage audit transitions.
pub fn apply_reconciliation(
    store: &Store,
    plan: &ReconciliationPlan,
) -> Result<(), ReconcileError> {
    for action in &plan.actions {
        match action {
            LifecycleAction::CreateExternalProposal
            | LifecycleAction::CleanupProposedExternal { .. } => {}
            LifecycleAction::Transition {
                to,
                reason,
                observed_at,
            } => {
                if store.candidate_state(&plan.candidate_id)? != *to {
                    store.transition_candidate(
                        &plan.candidate_id,
                        *to,
                        reason.as_str(),
                        *observed_at,
                    )?;
                }
            }
            LifecycleAction::UpsertExternalMapping(mapping) => {
                store.upsert_external_mapping(ExternalObjectMapping {
                    candidate_id: mapping.candidate_id.clone(),
                    source: mapping.source,
                    external_object_id: mapping.external_object_id.clone(),
                    external_source_id: mapping.external_source_id.clone(),
                    mapped_at: mapping.mapped_at,
                })?;
            }
        }
    }
    record_lifecycle_feedback(store, plan)?;
    Ok(())
}
