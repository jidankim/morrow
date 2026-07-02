#[path = "support/lifecycle.rs"]
mod lifecycle_support;

use lifecycle_support::{fresh_store, make_creating, make_visible, store_candidate_observed};
use morrow_storage::{CandidateKind, CandidateState};

#[test]
fn lifecycle_supersedes_prior_visible_candidate_by_same_anchor() {
    // Given: older queued, creating, and visible candidates plus unrelated candidates.
    let (_dir, store) = fresh_store("supersede-visible.sqlite");
    let queued = store_candidate_observed(
        &store,
        (
            CandidateKind::CalendarEvent,
            "msg-supersede",
            "2026-07-15T18:00:00Z",
        ),
        100,
    );
    let creating = store_candidate_observed(
        &store,
        (
            CandidateKind::EventUpdate,
            "msg-supersede",
            "2026-07-15T18:30:00Z",
        ),
        110,
    );
    make_creating(&store, &creating, 9);
    let visible = store_candidate_observed(
        &store,
        (
            CandidateKind::CalendarEvent,
            "msg-supersede",
            "2026-07-15T19:00:00Z",
        ),
        120,
    );
    make_visible(&store, &visible, 10);
    let other_family = store_candidate_observed(
        &store,
        (
            CandidateKind::TaskReminder,
            "msg-supersede",
            "2026-07-15T19:30:00Z",
        ),
        130,
    );
    let other_anchor = store_candidate_observed(
        &store,
        (
            CandidateKind::CalendarEvent,
            "msg-other-anchor",
            "2026-07-15T19:30:00Z",
        ),
        140,
    );
    let newer = store_candidate_observed(
        &store,
        (
            CandidateKind::EventReschedule,
            "msg-supersede",
            "2026-07-15T20:00:00Z",
        ),
        200,
    );

    // When: the newer candidate suppresses prior candidates for its anchor.
    let suppressed = store
        .supersede_prior_candidates_by_anchor(&newer, 20)
        .expect("supersede prior");

    // Then: only suppressible older same-anchor candidates in the calendar family move.
    assert_eq!(
        suppressed,
        vec![queued.clone(), creating.clone(), visible.clone()]
    );
    for candidate_id in [&queued, &creating, &visible] {
        assert_eq!(
            store
                .candidate_state(candidate_id)
                .expect("superseded state"),
            CandidateState::Suppressed
        );
    }
    assert_eq!(
        store.candidate_state(&newer).expect("newer state"),
        CandidateState::Queued
    );
    assert_eq!(
        store
            .candidate_state(&other_family)
            .expect("other family state"),
        CandidateState::Queued
    );
    assert_eq!(
        store
            .candidate_state(&other_anchor)
            .expect("other anchor state"),
        CandidateState::Queued
    );
    let audit = store.audit_entries(&visible).expect("visible audit");
    let supersede = audit.last().expect("supersede audit");
    assert_eq!(supersede.from_state, CandidateState::Visible);
    assert_eq!(supersede.to_state, CandidateState::Suppressed);
    assert_eq!(supersede.reason, "candidate_superseded");
}
