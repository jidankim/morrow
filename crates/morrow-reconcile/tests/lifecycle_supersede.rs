//! Lifecycle supersede regression tests.

use morrow_reconcile::{
    apply_reconciliation, reconcile_candidate, reconcile_same_anchor_supersede_candidates,
    CandidateLifecycle, ExternalItemObservation, LifecycleAction, LifecycleReason,
    ReconciliationPlan, Suppression,
};
use morrow_storage::{CandidateId, CandidateKind, CandidateState, Store};

/// Shared lifecycle test fixtures.
pub mod support;
use support::{db_path, visible_candidate, TestError};

#[test]
fn terminal_manual_lifecycle_candidates_are_protected_for_calendar_and_reminder_kinds(
) -> Result<(), Box<dyn std::error::Error>> {
    for kind in manual_lifecycle_kinds() {
        for state in [
            CandidateState::Completed,
            CandidateState::Rejected,
            CandidateState::Expired,
            CandidateState::Suppressed,
            CandidateState::Unknown,
            CandidateState::Failed,
        ] {
            let plan = reconcile_candidate(
                &visible_lifecycle(kind, state),
                &ExternalItemObservation::Pending,
            )?;

            assert!(plan.actions.is_empty());
            assert_eq!(
                plan.suppressions,
                [
                    Suppression::ManualChangeProposalOnly,
                    Suppression::ClosedCandidate,
                ]
            );
        }
    }
    Ok(())
}

#[test]
fn supersede_same_anchor_lifecycle_plan_suppresses_only_open_prior_same_family_candidates() {
    let newer = same_anchor_lifecycle(
        CandidateKind::EventReschedule,
        CandidateState::Queued,
        40,
        300,
    );
    let older_candidates = [
        (
            CandidateKind::CalendarEvent,
            CandidateState::Queued,
            10,
            100,
        ),
        (
            CandidateKind::EventUpdate,
            CandidateState::CreatingExternal,
            11,
            101,
        ),
        (
            CandidateKind::EventCancellation,
            CandidateState::Visible,
            12,
            102,
        ),
        (
            CandidateKind::ReminderReschedule,
            CandidateState::Visible,
            13,
            103,
        ),
        (
            CandidateKind::CalendarEvent,
            CandidateState::Approved,
            14,
            104,
        ),
        (
            CandidateKind::CalendarEvent,
            CandidateState::Completed,
            15,
            105,
        ),
        (
            CandidateKind::CalendarEvent,
            CandidateState::Rejected,
            16,
            106,
        ),
        (
            CandidateKind::CalendarEvent,
            CandidateState::Expired,
            17,
            107,
        ),
        (
            CandidateKind::CalendarEvent,
            CandidateState::Suppressed,
            18,
            108,
        ),
        (
            CandidateKind::CalendarEvent,
            CandidateState::Unknown,
            19,
            109,
        ),
        (
            CandidateKind::CalendarEvent,
            CandidateState::Failed,
            20,
            110,
        ),
    ]
    .map(|(kind, state, normalized_minute, observed_at)| {
        same_anchor_lifecycle(kind, state, normalized_minute, observed_at)
    });

    let plans = reconcile_same_anchor_supersede_candidates(&newer, &older_candidates);

    assert_eq!(plans.len(), older_candidates.len() - 1);
    assert_supersede_transition(&plans[0], &older_candidates[0], newer.observed_at);
    assert_supersede_transition(&plans[1], &older_candidates[1], newer.observed_at);
    assert_supersede_transition(&plans[2], &older_candidates[2], newer.observed_at);
    assert_protected(
        &plans[3],
        &older_candidates[4],
        Suppression::ApprovedMutationSuppressed,
    );
    for (plan, candidate) in plans[4..].iter().zip(&older_candidates[5..]) {
        assert_protected(plan, candidate, Suppression::ClosedCandidate);
    }
}

#[test]
fn lifecycle_supersede_plan_apply_twice_records_one_audit_and_no_feedback() -> Result<(), TestError>
{
    let store = Store::open(&db_path("supersede_plan_idempotent"))?;
    let older_id = visible_candidate(&store, CandidateKind::CalendarEvent, "supersede-old")?;
    let newer = same_anchor_lifecycle(
        CandidateKind::EventReschedule,
        CandidateState::Queued,
        40,
        300,
    );
    let older = CandidateLifecycle {
        candidate_id: older_id.clone(),
        kind: CandidateKind::CalendarEvent,
        state: CandidateState::Visible,
        mapping: None,
        observed_at: 100,
    };
    let plans = reconcile_same_anchor_supersede_candidates(&newer, &[older]);
    let plan = &plans[0];

    apply_reconciliation(&store, plan)?;
    apply_reconciliation(&store, plan)?;

    assert_eq!(
        store.candidate_state(&older_id)?,
        CandidateState::Suppressed
    );
    assert_eq!(plan.actions.len(), 1);
    assert_audit_reason_count(&store, &older_id, LifecycleReason::CandidateSuperseded, 1)?;
    let counts = store.feedback_eval_counts()?;
    assert_eq!(counts.label_count, 0);
    assert_eq!(counts.feature_snapshot_count, 0);
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

fn same_anchor_lifecycle(
    kind: CandidateKind,
    state: CandidateState,
    normalized_minute: u8,
    observed_at: i64,
) -> CandidateLifecycle {
    let normalized_time = format!("2026-07-01T10:{normalized_minute:02}:00Z");
    CandidateLifecycle {
        candidate_id: CandidateId::derive(kind, "same-chat", "same-message", &normalized_time),
        kind,
        state,
        mapping: None,
        observed_at,
    }
}

const fn manual_lifecycle_kinds() -> [CandidateKind; 6] {
    [
        CandidateKind::EventUpdate,
        CandidateKind::EventReschedule,
        CandidateKind::EventCancellation,
        CandidateKind::ReminderUpdate,
        CandidateKind::ReminderReschedule,
        CandidateKind::ReminderCancellation,
    ]
}

fn assert_supersede_transition(
    plan: &ReconciliationPlan,
    candidate: &CandidateLifecycle,
    observed_at: i64,
) {
    assert_eq!(plan.candidate_id, candidate.candidate_id);
    assert_eq!(
        plan.actions,
        [LifecycleAction::Transition {
            to: CandidateState::Suppressed,
            reason: LifecycleReason::CandidateSuperseded,
            observed_at,
        }]
    );
    assert!(plan.suppressions.is_empty());
}

fn assert_protected(
    plan: &ReconciliationPlan,
    candidate: &CandidateLifecycle,
    suppression: Suppression,
) {
    assert_eq!(plan.candidate_id, candidate.candidate_id);
    assert!(plan.actions.is_empty());
    assert_eq!(plan.suppressions, [suppression]);
}

fn assert_audit_reason_count(
    store: &Store,
    candidate_id: &CandidateId,
    reason: LifecycleReason,
    expected: usize,
) -> Result<(), TestError> {
    let actual = store
        .audit_entries(candidate_id)?
        .iter()
        .filter(|entry| entry.reason == reason.as_str())
        .count();
    assert_eq!(actual, expected);
    Ok(())
}
