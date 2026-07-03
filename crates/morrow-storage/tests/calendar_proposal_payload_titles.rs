use morrow_storage::{CandidateDraft, CandidateKind, CapPolicy, Store};

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

#[test]
fn calendar_proposal_payload_substitutes_generic_title_for_unproven_raw_titles() {
    // Given: a cap-selected calendar candidate with a raw title that does not match fixture filters.
    let (_dir, _db_path, store) = fresh_store("calendar-payload-raw-title.sqlite");
    let candidate = store
        .create_candidate(candidate_draft(
            CandidateKind::CalendarEvent,
            "msg-board-merger",
            "chat-board-merger",
            "Board merger call 5551234567",
            950,
            "2026-07-15T19:00:00Z",
        ))
        .expect("create raw title candidate");
    let cap_plan = store
        .apply_visibility_caps(CapPolicy::refill_for_pending(1, 0), 1_783_010_000)
        .expect("apply visibility caps");
    assert_eq!(cap_plan.visible.len(), 1);

    // When: storage loads replay payloads for the selected cap id.
    let payloads = store
        .calendar_proposal_payloads(std::slice::from_ref(&candidate))
        .expect("calendar replay payloads");

    // Then: the payload keeps the candidate but substitutes the known generic title.
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0].candidate_id, candidate);
    assert_eq!(payloads[0].title, "Messages event candidate");
    assert!(!format!("{:?}", payloads[0]).contains("5551234567"));
}

#[test]
fn calendar_proposal_payload_substitutes_generic_title_for_separated_phone_titles() {
    // Given
    let cases = [
        (
            "calendar-payload-phone-hyphen.sqlite",
            "Board call 555-111-2222",
        ),
        (
            "calendar-payload-phone-space.sqlite",
            "Board call 555 111 2222",
        ),
    ];

    for (db_name, title) in cases {
        let (_dir, _db_path, store) = fresh_store(db_name);
        let candidate = store
            .create_candidate(candidate_draft(
                CandidateKind::CalendarEvent,
                "msg-board-phone",
                "chat-board-phone",
                title,
                950,
                "2026-07-15T19:00:00Z",
            ))
            .expect("create separated phone title candidate");
        let cap_plan = store
            .apply_visibility_caps(CapPolicy::refill_for_pending(1, 0), 1_783_010_000)
            .expect("apply visibility caps");
        assert_eq!(cap_plan.visible.len(), 1);

        // When
        let payloads = store
            .calendar_proposal_payloads(std::slice::from_ref(&candidate))
            .expect("calendar replay payloads");

        // Then
        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].candidate_id, candidate);
        assert_eq!(payloads[0].title, "Messages event candidate");
        assert!(!format!("{:?}", payloads[0]).contains(title));
    }
}

#[test]
fn calendar_proposal_payload_preserves_safe_candidate_title() {
    // Given: a cap-selected calendar candidate with a short generated title.
    let (_dir, _db_path, store) = fresh_store("calendar-payload-safe-title.sqlite");
    let candidate = store
        .create_candidate(candidate_draft(
            CandidateKind::CalendarEvent,
            "msg-morrow-qa",
            "chat-morrow-qa",
            "Morrow QA",
            950,
            "2026-07-15T19:00:00Z",
        ))
        .expect("create candidate");
    let cap_plan = store
        .apply_visibility_caps(CapPolicy::refill_for_pending(1, 0), 1_783_010_000)
        .expect("apply visibility caps");
    assert_eq!(cap_plan.visible.len(), 1);

    // When
    let payloads = store
        .calendar_proposal_payloads(std::slice::from_ref(&candidate))
        .expect("calendar replay payloads");

    // Then
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0].title, "Morrow QA");
}
