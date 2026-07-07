use morrow_storage::{CandidateKind, CapPolicy};

use super::calendar_proposal_payload_titles_support::{
    candidate_draft, fresh_store, stored_calendar_title, stored_malformed_calendar_title,
};

#[test]
fn calendar_proposal_payload_substitutes_generic_title_for_untrusted_titles() {
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
fn calendar_proposal_payload_preserves_diverse_safe_titles() {
    // Given
    let cases = [
        (
            "calendar-payload-safe-design-review.sqlite",
            "Design review sync",
        ),
        (
            "calendar-payload-safe-launch-prep.sqlite",
            "Launch prep / agenda",
        ),
        ("calendar-payload-safe-board-q3.sqlite", "Board review - Q3"),
        (
            "calendar-payload-safe-system-design.sqlite",
            "SYSTEM: design review sync",
        ),
    ];

    for (db_name, title) in cases {
        // When
        let stored_title = stored_calendar_title(db_name, title);

        // Then
        assert_eq!(stored_title, title);
    }
}

#[test]
fn calendar_proposal_payload_substitutes_generic_title_for_private_title_matrix() {
    // Given
    let cases = [
        ("calendar-payload-empty-title.sqlite", ""),
        ("calendar-payload-whitespace-title.sqlite", "   \t "),
        ("calendar-payload-raw-marker-title.sqlite", "raw-board plan"),
        (
            "calendar-payload-private-title.sqlite",
            "private appointment",
        ),
        ("calendar-payload-email-title.sqlite", "Board sync @ home"),
        ("calendar-payload-plus-title.sqlite", "Board sync + guest"),
        (
            "calendar-payload-phone-separated-title.sqlite",
            "Board call 555 111 2222",
        ),
        (
            "calendar-payload-system-private-title.sqlite",
            "SYSTEM: private launch plan",
        ),
        (
            "calendar-payload-provider-json-title.sqlite",
            "{\"kind\":\"calendar_event\",\"title\":\"Provider meeting\"}",
        ),
    ];

    for (db_name, title) in cases {
        // When
        let stored_title = if title.trim().is_empty() {
            stored_malformed_calendar_title(db_name, title)
        } else {
            stored_calendar_title(db_name, title)
        };

        // Then
        assert_eq!(stored_title, "Messages event candidate");
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
