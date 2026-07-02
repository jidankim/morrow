use morrow_storage::{CandidateDraft, CandidateKind, CandidateState, StorageError, Store};

fn fresh_store(name: &str) -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join(name);
    let store = Store::open(&db_path).expect("open store");
    (dir, store)
}

fn lifecycle_draft(kind: CandidateKind) -> CandidateDraft {
    CandidateDraft {
        kind,
        chat_guid: "iMessage;+;+15555550100".to_owned(),
        anchor_message_guid: "msg-lifecycle".to_owned(),
        title: "Dentist appointment".to_owned(),
        confidence_millis: 850,
        normalized_time: "2026-07-15T19:00:00Z".to_owned(),
        evidence_excerpt: "dentist on July 15 at 7".to_owned(),
        observed_at: 1_783_000_000,
    }
}

#[test]
fn lifecycle_reschedule_and_cancel_kinds_persist_parse_audit_and_reject_backward_transitions() {
    for (kind, reason) in [
        (CandidateKind::EventReschedule, "candidate_rescheduled"),
        (CandidateKind::EventCancellation, "candidate_cancelled"),
        (CandidateKind::ReminderReschedule, "candidate_rescheduled"),
        (CandidateKind::ReminderCancellation, "candidate_cancelled"),
    ] {
        // Given: a lifecycle candidate kind stored through the normal candidate boundary.
        let (_dir, store) = fresh_store(kind.as_str());
        let candidate_id = store
            .create_candidate(lifecycle_draft(kind))
            .expect("create lifecycle candidate");

        // When: storage records the lifecycle outcome and rejects a stale backward move.
        store
            .transition_candidate(
                &candidate_id,
                CandidateState::Suppressed,
                reason,
                1_783_000_010,
            )
            .expect("record lifecycle outcome");
        let stale = store
            .transition_candidate(
                &candidate_id,
                CandidateState::Queued,
                "stale replay",
                1_783_000_011,
            )
            .expect_err("stale transition rejected");
        let readback = store
            .candidate_lifecycle_readback(&candidate_id)
            .expect("lifecycle readback");

        // Then: kind/state/reason parse back and the terminal lifecycle state never moves backward.
        assert!(matches!(stale, StorageError::InvalidTransition { .. }));
        assert_eq!(readback.kind, kind);
        assert_eq!(readback.state, CandidateState::Suppressed);
        assert_eq!(readback.current_reason, reason);
        assert_eq!(
            store
                .audit_entries(&candidate_id)
                .expect("audit")
                .last()
                .expect("last audit")
                .reason,
            reason
        );
    }
}
