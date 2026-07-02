#[path = "support/lifecycle.rs"]
mod lifecycle_support;

use lifecycle_support::{
    fresh_store, make_creating, make_visible, move_to_state, store_candidate,
    store_candidate_observed,
};
use morrow_storage::{CandidateKind, CandidateState};

#[test]
fn lifecycle_does_not_supersede_newer_same_anchor_candidate() {
    // Given: an older current candidate and newer queued, creating, and visible same-anchor candidates.
    let (_dir, store) = fresh_store("supersede-newer.sqlite");
    let older_current = store_candidate_observed(
        &store,
        (
            CandidateKind::CalendarEvent,
            "msg-newer-protected",
            "2026-07-15T17:00:00Z",
        ),
        100,
    );
    let newer_queued = store_candidate_observed(
        &store,
        (
            CandidateKind::EventUpdate,
            "msg-newer-protected",
            "2026-07-15T18:00:00Z",
        ),
        200,
    );
    let newer_creating = store_candidate_observed(
        &store,
        (
            CandidateKind::EventReschedule,
            "msg-newer-protected",
            "2026-07-15T19:00:00Z",
        ),
        210,
    );
    make_creating(&store, &newer_creating, 211);
    let newer_visible = store_candidate_observed(
        &store,
        (
            CandidateKind::EventCancellation,
            "msg-newer-protected",
            "2026-07-15T20:00:00Z",
        ),
        220,
    );
    make_visible(&store, &newer_visible, 221);

    // When: a stale/out-of-order supersede call runs for the older candidate.
    let suppressed = store
        .supersede_prior_candidates_by_anchor(&older_current, 300)
        .expect("supersede from older current");

    // Then: newer same-anchor candidates are not treated as prior candidates.
    assert!(suppressed.is_empty());
    assert_eq!(
        store
            .candidate_state(&older_current)
            .expect("older current state"),
        CandidateState::Queued
    );
    assert_eq!(
        store
            .candidate_state(&newer_queued)
            .expect("newer queued state"),
        CandidateState::Queued
    );
    assert_eq!(
        store
            .candidate_state(&newer_creating)
            .expect("newer creating state"),
        CandidateState::CreatingExternal
    );
    assert_eq!(
        store
            .candidate_state(&newer_visible)
            .expect("newer visible state"),
        CandidateState::Visible
    );
}

#[test]
fn lifecycle_supersedes_only_inserted_prior_candidate_when_same_anchor_timestamps_match() {
    // Given: three same-anchor candidates recorded with the same observation timestamp.
    let (_dir, store) = fresh_store("supersede-equal-created-at.sqlite");
    let prior = store_candidate_observed(
        &store,
        (
            CandidateKind::CalendarEvent,
            "msg-equal-created-at",
            "2026-07-15T17:00:00Z",
        ),
        100,
    );
    let current = store_candidate_observed(
        &store,
        (
            CandidateKind::EventReschedule,
            "msg-equal-created-at",
            "2026-07-15T18:00:00Z",
        ),
        100,
    );
    let later_same_timestamp = store_candidate_observed(
        &store,
        (
            CandidateKind::EventCancellation,
            "msg-equal-created-at",
            "2026-07-15T19:00:00Z",
        ),
        100,
    );

    // When: supersede runs for the middle inserted candidate.
    let suppressed = store
        .supersede_prior_candidates_by_anchor(&current, 300)
        .expect("supersede same timestamp");

    // Then: the stable insertion tie-breaker suppresses only the true prior row.
    assert_eq!(suppressed, vec![prior.clone()]);
    assert_eq!(
        store.candidate_state(&prior).expect("prior state"),
        CandidateState::Suppressed
    );
    assert_eq!(
        store.candidate_state(&current).expect("current state"),
        CandidateState::Queued
    );
    assert_eq!(
        store
            .candidate_state(&later_same_timestamp)
            .expect("later same timestamp state"),
        CandidateState::Queued
    );
}

#[test]
fn lifecycle_does_not_supersede_approved_or_terminal_candidates() {
    // Given: approved and terminal candidates sharing the newer candidate's anchor.
    let (_dir, store) = fresh_store("supersede-protected.sqlite");
    let protected_states = [
        CandidateState::Approved,
        CandidateState::Completed,
        CandidateState::Rejected,
        CandidateState::Expired,
        CandidateState::Suppressed,
        CandidateState::Unknown,
        CandidateState::Failed,
    ];
    let mut protected = Vec::new();
    for (index, state) in protected_states.into_iter().enumerate() {
        let normalized_time = format!("2026-07-15T{index:02}:00:00Z");
        let candidate_id = store_candidate(
            &store,
            (
                CandidateKind::CalendarEvent,
                "msg-protected",
                &normalized_time,
            ),
        );
        move_to_state(&store, &candidate_id, state);
        protected.push((candidate_id, state));
    }
    let newer = store_candidate(
        &store,
        (
            CandidateKind::EventCancellation,
            "msg-protected",
            "2026-07-15T21:00:00Z",
        ),
    );

    // When: same-anchor supersede runs for the newer candidate.
    let suppressed = store
        .supersede_prior_candidates_by_anchor(&newer, 20)
        .expect("supersede protected");

    // Then: approved and terminal candidates are left unchanged.
    assert!(suppressed.is_empty());
    for (candidate_id, state) in protected {
        assert_eq!(store.candidate_state(&candidate_id).expect("state"), state);
    }
}
