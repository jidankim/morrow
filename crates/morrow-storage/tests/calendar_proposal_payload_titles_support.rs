use std::process::Command;

use morrow_storage::{CandidateDraft, CandidateId, CandidateKind, CapPolicy, Store};

pub fn fresh_store(name: &str) -> (tempfile::TempDir, std::path::PathBuf, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join(name);
    let store = Store::open(&db_path).expect("open store");
    (dir, db_path, store)
}

pub fn candidate_draft(
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

pub fn stored_calendar_title(db_name: &str, title: &str) -> String {
    let (_dir, _db_path, store) = fresh_store(db_name);
    let candidate = store
        .create_candidate(candidate_draft(
            CandidateKind::CalendarEvent,
            "msg-title-policy",
            "chat-title-policy",
            title,
            950,
            "2026-07-15T19:00:00Z",
        ))
        .expect("create title policy candidate");
    let cap_plan = store
        .apply_visibility_caps(CapPolicy::refill_for_pending(1, 0), 1_783_010_000)
        .expect("apply visibility caps");
    assert_eq!(cap_plan.visible.len(), 1);

    let payloads = store
        .calendar_proposal_payloads(std::slice::from_ref(&candidate))
        .expect("calendar replay payloads");

    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0].candidate_id, candidate);
    payloads[0].title.clone()
}

pub fn stored_reminder_title(db_name: &str, title: &str) -> String {
    let (_dir, _db_path, store) = fresh_store(db_name);
    let candidate = store
        .create_candidate(candidate_draft(
            CandidateKind::TaskReminder,
            "msg-reminder-title-policy",
            "chat-reminder-title-policy",
            title,
            950,
            "2026-07-15T19:00:00Z",
        ))
        .expect("create reminder title policy candidate");
    let cap_plan = store
        .apply_visibility_caps(CapPolicy::refill_for_pending(1, 0), 1_783_010_000)
        .expect("apply visibility caps");
    assert_eq!(cap_plan.visible.len(), 1);

    let payloads = store
        .reminder_proposal_payloads(std::slice::from_ref(&candidate))
        .expect("reminder replay payloads");

    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0].candidate_id, candidate);
    payloads[0].title.clone()
}

pub fn stored_malformed_calendar_title(db_name: &str, title: &str) -> String {
    let (_dir, db_path, store) = fresh_store(db_name);
    let normalized_time = "2026-07-15T19:00:00Z";
    let candidate = CandidateId::derive(
        CandidateKind::CalendarEvent,
        "chat-title-policy",
        "msg-title-policy",
        normalized_time,
    );
    let output = Command::new("sqlite3")
        .arg("-batch")
        .arg(&db_path)
        .arg(format!(
            "INSERT INTO candidates
             (id, kind, state, chat_guid, anchor_message_guid, title, confidence_millis,
              normalized_time, candidate_version, current_reason, created_at, updated_at)
             VALUES ('{}', 'calendar_event', 'creating_external', 'chat-title-policy',
                     'msg-title-policy', '{}', 950, '{}', 1, 'test-fixture',
                     1783010000, 1783010000);",
            candidate.as_str(),
            title.replace('\'', "''"),
            normalized_time
        ))
        .output()
        .expect("run sqlite3");
    assert!(
        output.status.success(),
        "sqlite3 failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let payloads = store
        .calendar_proposal_payloads(std::slice::from_ref(&candidate))
        .expect("calendar replay payloads");

    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0].candidate_id, candidate);
    payloads[0].title.clone()
}
