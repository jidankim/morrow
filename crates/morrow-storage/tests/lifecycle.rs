use morrow_storage::{
    CandidateDraft, CandidateKind, CandidateState, ExternalObjectMapping, ExternalSource,
    QuietLogDraft, ReplayStream, StorageError, Store,
};

fn fresh_store(name: &str) -> (tempfile::TempDir, std::path::PathBuf, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join(name);
    let store = Store::open(&db_path).expect("open store");
    (dir, db_path, store)
}

#[test]
fn open_creates_missing_app_data_directory_before_sqlite_initializes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir
        .path()
        .join("Application Support")
        .join("dev.morrow.desktop")
        .join("morrow.sqlite");
    let app_data_dir = db_path.parent().expect("db path has parent");

    assert!(!app_data_dir.exists());
    let store = Store::open(&db_path)
        .expect("open store should create the app data directory before sqlite runs");

    assert!(app_data_dir.is_dir());
    assert!(db_path.is_file());
    assert!(store
        .table_names()
        .expect("schema tables")
        .contains(&"candidates".to_owned()));
}

fn event_draft(anchor: &str) -> CandidateDraft {
    CandidateDraft {
        kind: CandidateKind::CalendarEvent,
        chat_guid: "iMessage;+;+15555550100".to_owned(),
        anchor_message_guid: anchor.to_owned(),
        title: "Dentist appointment".to_owned(),
        confidence_millis: 850,
        normalized_time: "2026-07-15T19:00:00Z".to_owned(),
        evidence_excerpt: "dentist on July 15 at 7".to_owned(),
        observed_at: 1_783_000_000,
    }
}

#[test]
fn malformed_candidate_normalized_time_is_rejected() {
    // Given: a candidate draft with an impossible calendar date.
    let (_dir, _db_path, store) = fresh_store("bad-normalized-time.sqlite");
    let mut draft = event_draft("msg-bad-time");
    draft.normalized_time = "2026-02-31T10:00:00Z".to_owned();

    // When: the candidate crosses the storage boundary.
    let err = store
        .create_candidate(draft)
        .expect_err("candidate normalized_time should be parsed");

    // Then: storage rejects it as an invalid normalized_time value.
    match err {
        StorageError::InvalidInput { field, .. } => {
            assert_eq!(field, "normalized_time");
        }
        other => panic!("expected invalid normalized_time, got {other}"),
    }
}

#[test]
fn normalized_time_rejects_suffix_seconds_timezone_and_overlong_text() {
    for normalized_time in [
        "2026-07-01T10:00:00 private prompt text",
        "2026-07-01T10:00:abZ",
        "2026-07-01T10:00:00[Not A Zone]",
        "2026-07-01T10:00:00[America/Prompt]",
        "2026-07-01T10:00:00[America/Response]",
        "2026-07-01T10:00:00[America/Full_message]",
        "2026-07-01T10:00:00[America/Private_clinic_visit]",
        "2026-07-01T10:00:00[America/PromptText]",
        "2026-07-01T10:00:00[America/ResponseText]",
        "2026-07-01T10:00:00[America/FullMessage]",
        "2026-07-01T10:00:00[America/RawMessage]",
        "2026-07-01T10:00:00[America/MessageBody]",
        "2026-07-01T10:00:00[America/PrivateClinicVisit]",
        "2026-07-01T10:00:00Z trailing",
        "2026-07-01T10:00:00Zabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz",
    ] {
        let (_dir, _db_path, store) = fresh_store("bad-normalized-time-suffix.sqlite");
        let mut draft = event_draft("msg-bad-time-suffix");
        draft.normalized_time = normalized_time.to_owned();

        let err = store
            .create_candidate(draft)
            .expect_err("normalized_time suffix must be rejected");

        match err {
            StorageError::InvalidInput { field, .. } => {
                assert_eq!(field, "normalized_time");
            }
            other => panic!("expected invalid normalized_time, got {other}"),
        }
    }
}

#[test]
fn audited_candidate_transition_when_candidate_becomes_visible() {
    // Given: a queued candidate in a migrated database.
    let (_dir, _db_path, store) = fresh_store("audit.sqlite");
    let candidate_id = store
        .create_candidate(event_draft("msg-a"))
        .expect("create");

    // When: the candidate moves through external creation to visible.
    store
        .transition_candidate(
            &candidate_id,
            CandidateState::CreatingExternal,
            "calendar write started",
            1_783_000_001,
        )
        .expect("transition creating");
    store
        .transition_candidate(
            &candidate_id,
            CandidateState::Visible,
            "proposal visible in Morrow Proposed",
            1_783_000_002,
        )
        .expect("transition visible");

    // Then: every transition is audited with from/to state and reason.
    let audit = store.audit_entries(&candidate_id).expect("audit");
    assert_eq!(audit.len(), 2);
    assert_eq!(audit[0].from_state, CandidateState::Queued);
    assert_eq!(audit[0].to_state, CandidateState::CreatingExternal);
    assert_eq!(audit[0].reason, "calendar write started");
    assert_eq!(audit[1].from_state, CandidateState::CreatingExternal);
    assert_eq!(audit[1].to_state, CandidateState::Visible);
}

#[test]
fn malformed_candidate_state_transition_is_rejected_when_state_goes_backwards() {
    // Given: a visible candidate.
    let (_dir, _db_path, store) = fresh_store("bad-state.sqlite");
    let candidate_id = store
        .create_candidate(event_draft("msg-b"))
        .expect("create");
    store
        .transition_candidate(&candidate_id, CandidateState::CreatingExternal, "start", 10)
        .expect("transition creating");
    store
        .transition_candidate(&candidate_id, CandidateState::Visible, "visible", 11)
        .expect("transition visible");

    // When: a stale replay tries to move it back to queued.
    let err = store
        .transition_candidate(&candidate_id, CandidateState::Queued, "stale replay", 12)
        .expect_err("stale transition rejected");

    // Then: the typed error names the invalid transition.
    assert!(matches!(err, StorageError::InvalidTransition { .. }));
}

#[test]
fn quiet_logs_expire_after_thirty_days_and_reject_full_message_payloads() {
    // Given: a quiet log containing only a short excerpt.
    let (_dir, _db_path, store) = fresh_store("quiet.sqlite");
    let created_at = 1_783_000_000;
    store
        .record_quiet_log(QuietLogDraft {
            chat_guid: "chat-quiet".to_owned(),
            anchor_message_guid: "quiet-msg".to_owned(),
            reason: "low confidence".to_owned(),
            excerpt: "maybe coffee next week".to_owned(),
            created_at,
        })
        .expect("quiet log");

    // When: a full-message-like payload is offered and retention runs after 31 days.
    let payload_err = store
        .record_quiet_log(QuietLogDraft {
            chat_guid: "chat-quiet".to_owned(),
            anchor_message_guid: "quiet-full".to_owned(),
            reason: "privacy probe".to_owned(),
            excerpt: "From: Alice\nTo: Bob\nDate: 2026-07-01\n\nCan you send the full thread?"
                .to_owned(),
            created_at,
        })
        .expect_err("full message rejected");
    let expired = store
        .expire_quiet_logs(created_at + 31 * 24 * 60 * 60)
        .expect("expire");

    // Then: the payload is rejected and old quiet logs are removed.
    assert!(matches!(payload_err, StorageError::PrivacyViolation { .. }));
    assert_eq!(expired, 1);
    assert_eq!(store.quiet_log_count().expect("count"), 0);
}

#[test]
fn restart_replay_does_not_duplicate_proposals_when_external_mapping_exists() {
    // Given: a visible candidate pending replay.
    let (_dir, db_path, store) = fresh_store("replay.sqlite");
    let candidate_id = store
        .create_candidate(event_draft("msg-c"))
        .expect("create");
    store
        .transition_candidate(&candidate_id, CandidateState::CreatingExternal, "start", 20)
        .expect("transition creating");
    store
        .transition_candidate(&candidate_id, CandidateState::Visible, "visible", 21)
        .expect("transition visible");
    let before_restart = store
        .next_replay_candidates(ReplayStream::CalendarProposals, 10)
        .expect("initial replay");
    assert_eq!(before_restart, vec![candidate_id.clone()]);

    // When: the process restarts after the external proposal has been mapped.
    store
        .upsert_external_mapping(ExternalObjectMapping {
            candidate_id: candidate_id.clone(),
            source: ExternalSource::Calendar,
            external_object_id: "eventkit://calendar/proposed/abc123".to_owned(),
            external_source_id: "calendar-source-primary".to_owned(),
            mapped_at: 22,
        })
        .expect("mapping");
    drop(store);
    let restarted = Store::open(&db_path).expect("restart");
    let after_restart = restarted
        .next_replay_candidates(ReplayStream::CalendarProposals, 10)
        .expect("replay after restart");

    // Then: the mapped proposal is not replayed again.
    assert!(after_restart.is_empty());
}

#[test]
fn external_mapping_rejects_conflicting_external_object_when_replayed() {
    // Given: two candidates and an existing external object mapping.
    let (_dir, _db_path, store) = fresh_store("mapping.sqlite");
    let first = store.create_candidate(event_draft("msg-d")).expect("first");
    let second = store
        .create_candidate(event_draft("msg-e"))
        .expect("second");
    let external_object_id = "eventkit://calendar/proposed/shared".to_owned();
    store
        .upsert_external_mapping(ExternalObjectMapping {
            candidate_id: first,
            source: ExternalSource::Calendar,
            external_object_id: external_object_id.clone(),
            external_source_id: "calendar-source-primary".to_owned(),
            mapped_at: 30,
        })
        .expect("first mapping");

    // When: replay tries to attach the same external object to another candidate.
    let err = store
        .upsert_external_mapping(ExternalObjectMapping {
            candidate_id: second,
            source: ExternalSource::Calendar,
            external_object_id,
            external_source_id: "calendar-source-primary".to_owned(),
            mapped_at: 31,
        })
        .expect_err("conflict");

    // Then: the conflict is rejected instead of duplicating ownership.
    assert!(matches!(err, StorageError::ExternalMappingConflict { .. }));
}
