use morrow_storage::{
    delete_all_at, plan_visibility, CandidateDraft, CandidateKind, CandidateState, CapPolicy,
    DeleteAllConfirmation, QueuedProposal, Store,
};

fn fresh_store(name: &str) -> (tempfile::TempDir, std::path::PathBuf, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join(name);
    let store = Store::open(&db_path).expect("open store");
    (dir, db_path, store)
}

fn event_draft(anchor: &str, chat_guid: &str, confidence_millis: i64) -> CandidateDraft {
    CandidateDraft {
        kind: CandidateKind::CalendarEvent,
        chat_guid: chat_guid.to_owned(),
        anchor_message_guid: anchor.to_owned(),
        title: "Follow up".to_owned(),
        confidence_millis,
        normalized_time: format!("2026-07-15T{:02}:00:00Z", 10 + confidence_millis % 10),
        evidence_excerpt: "meet on July 15".to_owned(),
        observed_at: 1_783_000_000,
    }
}

#[test]
fn delete_all_requires_exact_confirmation_and_removes_morrow_database() {
    // Given
    let (_dir, db_path, store) = fresh_store("delete-all.sqlite");
    store
        .create_candidate(event_draft("msg-a", "chat-a", 850))
        .expect("create candidate");

    // When
    let wrong_confirmation = DeleteAllConfirmation::parse("delete morrow data");
    let confirmation = DeleteAllConfirmation::parse("DELETE MORROW DATA").expect("confirmation");
    store.delete_all(confirmation).expect("delete all");

    // Then
    assert!(wrong_confirmation.is_err());
    assert!(!db_path.exists());
}

#[test]
fn delete_all_removes_morrow_sqlite_sidecars_and_keeps_unrelated_siblings() {
    // Given
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("morrow.sqlite");
    let wal_path = dir.path().join("morrow.sqlite-wal");
    let shm_path = dir.path().join("morrow.sqlite-shm");
    let journal_path = dir.path().join("morrow.sqlite-journal");
    let unrelated_path = dir.path().join("morrow.sqlite.backup");
    std::fs::write(&db_path, b"database").expect("write database");
    std::fs::write(&wal_path, b"wal").expect("write wal");
    std::fs::write(&shm_path, b"shm").expect("write shm");
    std::fs::write(&journal_path, b"journal").expect("write journal");
    std::fs::write(&unrelated_path, b"backup").expect("write unrelated sibling");
    let confirmation = DeleteAllConfirmation::parse("DELETE MORROW DATA").expect("confirmation");

    // When
    let receipt = delete_all_at(&db_path, confirmation).expect("delete all");

    // Then
    assert!(receipt.database_deleted);
    assert!(!db_path.exists());
    assert!(!wal_path.exists());
    assert!(!shm_path.exists());
    assert!(!journal_path.exists());
    assert!(unrelated_path.exists());
}

#[test]
fn invalid_delete_all_confirmation_preserves_database_files() {
    // Given
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("morrow.sqlite");
    let wal_path = dir.path().join("morrow.sqlite-wal");
    std::fs::write(&db_path, b"database").expect("write database");
    std::fs::write(&wal_path, b"wal").expect("write wal");

    // When
    let confirmation = DeleteAllConfirmation::parse("delete morrow data");

    // Then
    assert!(confirmation.is_err());
    assert!(db_path.exists());
    assert!(wal_path.exists());
}

#[test]
fn cap_planner_prioritizes_liked_high_confidence_earliest_and_per_chat_caps() {
    // Given
    let policy = CapPolicy::onboarding_backfill();
    let proposals = vec![
        QueuedProposal::new(
            "morrow_0000000000000001",
            "chat-a",
            900,
            "2026-07-20T10:00:00Z",
            false,
        )
        .expect("proposal"),
        QueuedProposal::new(
            "morrow_0000000000000002",
            "chat-a",
            910,
            "2026-07-21T10:00:00Z",
            false,
        )
        .expect("proposal"),
        QueuedProposal::new(
            "morrow_0000000000000003",
            "chat-a",
            920,
            "2026-07-22T10:00:00Z",
            false,
        )
        .expect("proposal"),
        QueuedProposal::new(
            "morrow_0000000000000004",
            "chat-a",
            930,
            "2026-07-23T10:00:00Z",
            false,
        )
        .expect("proposal"),
        QueuedProposal::new(
            "morrow_0000000000000005",
            "chat-a",
            940,
            "2026-07-24T10:00:00Z",
            false,
        )
        .expect("proposal"),
        QueuedProposal::new(
            "morrow_0000000000000006",
            "chat-a",
            950,
            "2026-07-25T10:00:00Z",
            false,
        )
        .expect("proposal"),
        QueuedProposal::new(
            "morrow_0000000000000007",
            "chat-b",
            700,
            "2026-07-10T10:00:00Z",
            true,
        )
        .expect("proposal"),
        QueuedProposal::new(
            "morrow_0000000000000008",
            "chat-b",
            700,
            "2026-07-09T10:00:00Z",
            false,
        )
        .expect("proposal"),
    ];

    // When
    let plan = plan_visibility(&proposals, policy);

    // Then
    let visible_ids = plan
        .visible
        .iter()
        .map(|proposal| proposal.candidate_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(visible_ids.first(), Some(&"morrow_0000000000000007"));
    assert_eq!(visible_ids.len(), 7);
    assert!(plan
        .deferred
        .iter()
        .any(|proposal| proposal.candidate_id.as_str() == "morrow_0000000000000001"));
}

#[test]
fn store_applies_refill_cap_to_real_queued_candidates() {
    // Given
    let (_dir, _db_path, store) = fresh_store("cap-application.sqlite");
    let low = store
        .create_candidate(event_draft("msg-low", "chat-a", 600))
        .expect("create low");
    let high = store
        .create_candidate(event_draft("msg-high", "chat-a", 950))
        .expect("create high");
    let other_chat = store
        .create_candidate(event_draft("msg-other", "chat-b", 700))
        .expect("create other chat");

    // When
    let plan = store
        .apply_visibility_caps(CapPolicy::refill_for_pending(2, 1), 1_783_000_100)
        .expect("apply caps");

    // Then
    assert_eq!(
        plan.visible
            .iter()
            .map(|proposal| proposal.candidate_id.as_str())
            .collect::<Vec<_>>(),
        vec![high.as_str()]
    );
    assert_eq!(
        store.candidate_state(&high).expect("high state"),
        CandidateState::CreatingExternal
    );
    assert_eq!(
        store.candidate_state(&low).expect("low state"),
        CandidateState::Queued
    );
    assert_eq!(
        store.candidate_state(&other_chat).expect("other state"),
        CandidateState::Queued
    );
}
