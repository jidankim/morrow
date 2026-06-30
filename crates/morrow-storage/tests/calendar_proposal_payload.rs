use morrow_storage::{
    CandidateDraft, CandidateId, CandidateKind, CandidateState, CapPolicy, StorageError, Store,
};

fn fresh_store(name: &str) -> (tempfile::TempDir, std::path::PathBuf, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join(name);
    let store = Store::open(&db_path).expect("open store");
    (dir, db_path, store)
}

fn candidate_draft(
    kind: CandidateKind,
    anchor: &str,
    chat_guid: &str,
    title: &str,
    confidence_millis: i64,
    normalized_time: &str,
) -> CandidateDraft {
    CandidateDraft {
        kind,
        chat_guid: chat_guid.to_owned(),
        anchor_message_guid: anchor.to_owned(),
        title: title.to_owned(),
        confidence_millis,
        normalized_time: normalized_time.to_owned(),
        evidence_excerpt: "source excerpt hidden".to_owned(),
        observed_at: 1_783_000_000 + confidence_millis,
    }
}

fn mutation_draft(anchor: &str, chat_guid: &str) -> CandidateDraft {
    candidate_draft(
        CandidateKind::EventReschedule,
        anchor,
        chat_guid,
        "Messages event candidate",
        990,
        "2026-07-15T20:00:00Z",
    )
}

fn forbidden_tokens(payload_debug: &str) -> Vec<&'static str> {
    let tokens = vec![
        "private clinic",
        "raw-chat",
        "raw-message",
        "+1555",
        "@example",
        "participant-secret",
        "evidence excerpt",
    ];
    for token in &tokens {
        assert!(
            !payload_debug.contains(token),
            "payload leaked forbidden token {token}: {payload_debug}"
        );
    }
    tokens
}

#[test]
fn calendar_proposal_payload_cap_selection_keeps_existing_ranking_fields_stable() {
    // Given: queued proposals whose ordering depends only on existing ranking fields.
    let (_dir, _db_path, store) = fresh_store("calendar-payload-cap-baseline.sqlite");
    let lower = store
        .create_candidate(candidate_draft(
            CandidateKind::CalendarEvent,
            "msg-low",
            "chat-a",
            "Messages event candidate",
            600,
            "2026-07-16T19:00:00Z",
        ))
        .expect("create lower candidate");
    let selected = store
        .create_candidate(candidate_draft(
            CandidateKind::CalendarEvent,
            "msg-selected",
            "chat-a",
            "Messages event candidate",
            950,
            "2026-07-15T19:00:00Z",
        ))
        .expect("create selected candidate");
    let other_chat = store
        .create_candidate(candidate_draft(
            CandidateKind::CalendarEvent,
            "msg-other",
            "chat-b",
            "Messages event candidate",
            700,
            "2026-07-14T19:00:00Z",
        ))
        .expect("create other chat candidate");

    // When: refill caps leave one slot for queued proposal promotion.
    let plan = store
        .apply_visibility_caps(CapPolicy::refill_for_pending(2, 1), 1_783_010_000)
        .expect("apply visibility caps");

    // Then: the highest-confidence queued candidate is selected and other candidates stay queued.
    assert_eq!(
        plan.visible
            .iter()
            .map(|proposal| proposal.candidate_id.as_str())
            .collect::<Vec<_>>(),
        vec![selected.as_str()]
    );
    assert_eq!(
        store.candidate_state(&selected).expect("selected state"),
        CandidateState::CreatingExternal
    );
    assert_eq!(
        store.candidate_state(&lower).expect("lower state"),
        CandidateState::Queued
    );
    assert_eq!(
        store.candidate_state(&other_chat).expect("other state"),
        CandidateState::Queued
    );
}

#[test]
fn calendar_proposal_payload_loads_only_sanitized_cap_selected_events() {
    // Given: cap-selected calendar and mutation candidates with raw identifiers only in storage internals.
    let (_dir, _db_path, store) = fresh_store("calendar-payload-sanitized.sqlite");
    let calendar = store
        .create_candidate(candidate_draft(
            CandidateKind::CalendarEvent,
            "raw-message-private-clinic",
            "raw-chat-+15555550100-alice@example.com-participant-secret",
            "Messages event candidate",
            950,
            "2026-07-15T19:00:00Z",
        ))
        .expect("create calendar candidate");
    let mutation = store
        .create_candidate(mutation_draft(
            "raw-message-reschedule-private-clinic",
            "raw-chat-+15555550101-bob@example.com-participant-secret",
        ))
        .expect("create mutation candidate");
    let malformed_time_error = store
        .create_candidate(candidate_draft(
            CandidateKind::CalendarEvent,
            "raw-message-malformed-time",
            "raw-chat-malformed-time",
            "Messages event candidate",
            940,
            "private clinic tomorrow",
        ))
        .expect_err("reject malformed time candidate");
    let raw_title = store
        .create_candidate(candidate_draft(
            CandidateKind::CalendarEvent,
            "raw-message-raw-title",
            "raw-chat-raw-title",
            "private clinic",
            930,
            "2026-07-16T19:00:00[Asia/Seoul]",
        ))
        .expect("create raw title candidate");
    let cap_plan = store
        .apply_visibility_caps(CapPolicy::refill_for_pending(3, 0), 1_783_010_000)
        .expect("apply visibility caps");
    assert_eq!(cap_plan.visible.len(), 3);
    assert!(matches!(
        malformed_time_error,
        StorageError::InvalidInput {
            field: "normalized_time",
            ..
        }
    ));

    // When: storage loads replay payloads for selected cap ids, including stale ids.
    let mut selected_ids = cap_plan
        .visible
        .iter()
        .map(|proposal| proposal.candidate_id.clone())
        .collect::<Vec<_>>();
    selected_ids.push(CandidateId::from_storage("morrow_0000000000000999").expect("stale id"));
    let payloads = store
        .calendar_proposal_payloads(&selected_ids)
        .expect("calendar replay payloads");

    // Then: only the sanitized calendar event payload is exposed for replay.
    assert_eq!(
        store.candidate_state(&mutation).expect("mutation state"),
        CandidateState::CreatingExternal
    );
    assert_eq!(
        store.candidate_state(&raw_title).expect("raw title state"),
        CandidateState::CreatingExternal
    );
    assert_eq!(payloads.len(), 2);
    assert!(payloads
        .iter()
        .any(|payload| payload.candidate_id == calendar
            && payload.normalized_time == "2026-07-15T19:00:00Z"));
    assert!(payloads
        .iter()
        .any(|payload| payload.candidate_id == raw_title
            && payload.normalized_time == "2026-07-16T19:00:00[Asia/Seoul]"));
    for payload in &payloads {
        assert_eq!(payload.kind, CandidateKind::CalendarEvent);
        assert_eq!(payload.title, "Messages event candidate");
        assert_eq!(payload.source_id, "morrow-selected-messages");
    }
    let payload_debug = format!("{payloads:?}");
    let tokens = forbidden_tokens(&payload_debug);
    println!(
        "calendar_proposal_payload sanitized payload_count={} source={} forbidden_tokens_checked={}",
        payloads.len(),
        payloads[0].source_id,
        tokens.len()
    );
}
