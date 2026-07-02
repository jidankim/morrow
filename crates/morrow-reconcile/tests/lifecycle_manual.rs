//! Manual lifecycle regression tests.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use morrow_reconcile::{
    apply_reconciliation, reconcile_candidate, CandidateLifecycle, ExternalItemObservation,
    LifecycleAction, LifecycleReason, Suppression,
};
use morrow_storage::{CandidateDraft, CandidateId, CandidateKind, CandidateState, Store};

static NEXT_DB: AtomicU64 = AtomicU64::new(1);

#[test]
fn records_reschedule_and_cancel_lifecycle_without_external_mutation(
) -> Result<(), Box<dyn std::error::Error>> {
    let store = Store::open(&db_path("records_reschedule_cancel_lifecycle"))?;
    for (index, (kind, reason, observed_at)) in [
        (
            CandidateKind::EventReschedule,
            LifecycleReason::CandidateRescheduled,
            90,
        ),
        (
            CandidateKind::ReminderReschedule,
            LifecycleReason::CandidateRescheduled,
            91,
        ),
        (
            CandidateKind::EventCancellation,
            LifecycleReason::CandidateCancelled,
            92,
        ),
        (
            CandidateKind::ReminderCancellation,
            LifecycleReason::CandidateCancelled,
            93,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        for state in [
            CandidateState::Queued,
            CandidateState::CreatingExternal,
            CandidateState::Visible,
        ] {
            let plan = reconcile_candidate(
                &visible_lifecycle(kind, state),
                &ExternalItemObservation::Pending,
            )?;

            assert_eq!(
                plan.actions,
                [LifecycleAction::Transition {
                    to: CandidateState::Suppressed,
                    reason,
                    observed_at: 10,
                }]
            );
            assert_eq!(plan.suppressions, [Suppression::ManualChangeProposalOnly]);
            assert!(!plan.actions.iter().any(|action| matches!(
                action,
                LifecycleAction::UpsertExternalMapping(_)
                    | LifecycleAction::CleanupProposedExternal { .. }
                    | LifecycleAction::CreateExternalProposal
            )));
        }

        let candidate_id = visible_candidate(&store, kind, &format!("manual-{index}"))?;
        let plan = reconcile_candidate(
            &CandidateLifecycle {
                candidate_id: candidate_id.clone(),
                kind,
                state: CandidateState::Visible,
                mapping: None,
                observed_at,
            },
            &ExternalItemObservation::Pending,
        )?;
        apply_reconciliation(&store, &plan)?;
        apply_reconciliation(&store, &plan)?;
        assert_eq!(
            store.candidate_state(&candidate_id)?,
            CandidateState::Suppressed
        );
        assert_audit_reason_count(&store, &candidate_id, reason, 1)?;
        assert_feedback_absent(&store)?;
    }
    Ok(())
}

#[test]
fn records_plan_fixed_lifecycle_reason_strings() {
    assert_eq!(
        LifecycleReason::CandidateSuperseded.as_str(),
        "candidate_superseded"
    );
    assert_eq!(
        LifecycleReason::CandidateRescheduled.as_str(),
        "candidate_rescheduled"
    );
    assert_eq!(
        LifecycleReason::CandidateCancelled.as_str(),
        "candidate_cancelled"
    );
}

#[test]
fn approved_candidates_suppress_phase3_manual_lifecycle_mutations(
) -> Result<(), Box<dyn std::error::Error>> {
    for kind in [
        CandidateKind::EventUpdate,
        CandidateKind::EventReschedule,
        CandidateKind::EventCancellation,
        CandidateKind::ReminderUpdate,
        CandidateKind::ReminderReschedule,
        CandidateKind::ReminderCancellation,
    ] {
        let plan = reconcile_candidate(
            &visible_lifecycle(kind, CandidateState::Approved),
            &ExternalItemObservation::approved_by_move("real-object", "source-main"),
        )?;

        assert!(plan.actions.is_empty());
        assert_eq!(
            plan.suppressions,
            [
                Suppression::ManualChangeProposalOnly,
                Suppression::ApprovedMutationSuppressed,
            ]
        );
    }
    Ok(())
}

fn visible_lifecycle(kind: CandidateKind, state: CandidateState) -> CandidateLifecycle {
    CandidateLifecycle {
        candidate_id: CandidateId::derive(kind, "chat", "message", "2026-07-01T10:00:00Z"),
        kind,
        state,
        mapping: None,
        observed_at: 10,
    }
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
        2,
    )?;
    store.transition_candidate(&candidate_id, CandidateState::Visible, "setup_visible", 3)?;
    Ok(candidate_id)
}

fn draft(kind: CandidateKind, suffix: &str) -> CandidateDraft {
    CandidateDraft {
        kind,
        chat_guid: format!("chat-{suffix}"),
        anchor_message_guid: format!("message-{suffix}"),
        title: format!("Candidate {suffix}"),
        confidence_millis: 850,
        normalized_time: format!("2026-07-01T10:{:02}:00Z", suffix.len()),
        evidence_excerpt: "See you then".to_owned(),
        observed_at: 1,
    }
}

fn assert_audit_reason_count(
    store: &Store,
    candidate_id: &CandidateId,
    reason: LifecycleReason,
    expected: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let entries = store.audit_entries(candidate_id)?;
    let actual = entries
        .iter()
        .filter(|entry| entry.reason == reason.as_str())
        .count();
    assert_eq!(actual, expected);
    Ok(())
}

fn assert_feedback_absent(store: &Store) -> Result<(), Box<dyn std::error::Error>> {
    let counts = store.feedback_eval_counts()?;
    assert_eq!(counts.label_count, 0);
    assert_eq!(counts.feature_snapshot_count, 0);
    Ok(())
}

fn db_path(name: &str) -> PathBuf {
    let next = NEXT_DB.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "morrow-reconcile-{name}-{}-{next}.sqlite3",
        std::process::id()
    ))
}
