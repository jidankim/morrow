use morrow_lib::native_bridge::ScanSelectedChatsRequest;
use morrow_storage::{CandidateState, ReplayStream, Store};
use serde_json::json;

#[path = "consent/list_reminder_profile.rs"]
mod list_reminder_profile;

use super::support::{
    assert_counts, candidate_state, chat, fake_state, native_batch, scan_request,
    scan_storage_dump, selected_chat_json, temp_db,
};

#[test]
fn native_scan_hides_source_excerpts_and_applies_caps() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan.sqlite")?;
    let request = scan_request(
        &[
            chat("design-partners", 3, &["p1", "p2", "p3"]),
            chat("ops-triage", 3, &["p1", "p2", "p3"]),
        ],
        &["design-partners"],
        false,
        1,
        0,
    )?;

    // When
    let result = fake_state(&db_path, native_batch()?)
        .scan_selected_chats_at(request, &db_path, &db_path)
        .map_err(|error| error.to_string())?;

    // Then
    assert_counts(&result, (1, 1, 1, 1, 0));
    let store = Store::open(&db_path).map_err(|error| error.to_string())?;
    assert_eq!(candidate_state(&store, &result)?, CandidateState::Visible);
    assert!(store
        .next_replay_candidates(ReplayStream::CalendarProposals, 10)
        .map_err(|error| error.to_string())?
        .is_empty());
    assert_eq!(
        store
            .privacy_summary()
            .map_err(|error| error.to_string())?
            .max_excerpt_len,
        "Source excerpt hidden by settings.".len()
    );
    Ok(())
}

#[test]
fn native_scan_includes_source_excerpts_when_enabled() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-source-excerpts.sqlite")?;
    let request = scan_request(
        &[
            chat("design-partners", 3, &["p1", "p2", "p3"]),
            chat("ops-triage", 3, &["p1", "p2", "p3"]),
        ],
        &[],
        true,
        1,
        0,
    )?;

    // When
    let result = fake_state(&db_path, native_batch()?)
        .scan_selected_chats_at(request, &db_path, &db_path)
        .map_err(|error| error.to_string())?;

    // Then
    assert_counts(&result, (1, 1, 1, 1, 0));
    let stored = scan_storage_dump(&db_path)?;
    assert!(
        stored.contains("Let's meet 2026-07-15 14:00 at the private clinic."),
        "{stored}"
    );
    assert!(
        stored.contains("The private clinic status update was funny."),
        "{stored}"
    );
    assert!(
        !stored.contains("Source excerpt hidden by settings."),
        "{stored}"
    );
    Ok(())
}

#[test]
fn native_scan_rejects_missing_feedback_text_snapshot_consent() {
    // Given
    let request = json!({
        "selectedChatIds": ["design-partners"],
        "selectedChats": [selected_chat_json("design-partners", 3, &["p1", "p2", "p3"])],
        "referenceTimezone": "Asia/Seoul",
        "referenceUnixSeconds": 1_782_352_400,
        "backfillPromptChatIds": [],
        "sourceExcerptsEnabled": true,
        "listReminderProfile": list_reminder_profile::disabled_list_reminder_profile_json(),
        "localDiagnosticsEnabled": false,
        "localDiagnosticsRetentionDays": 30,
        "capPolicy": {
            "mode": "refillForPending",
            "maxVisible": 1,
            "pendingCount": 0,
        },
    });

    // When / Then
    assert!(serde_json::from_value::<ScanSelectedChatsRequest>(request).is_err());
}

#[test]
fn native_scan_disabled_list_reminder_profile_preserves_existing_scan_behavior(
) -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-disabled-list-profile.sqlite")?;
    let request = scan_request(
        &[chat("design-partners", 3, &["p1", "p2", "p3"])],
        &[],
        true,
        1,
        0,
    )?;

    // When
    let result = fake_state(&db_path, native_batch()?)
        .scan_selected_chats_at(request, &db_path, &db_path)
        .map_err(|error| error.to_string())?;

    // Then
    assert_counts(&result, (1, 1, 0, 1, 0));
    Ok(())
}

#[test]
fn native_scan_redacts_when_feedback_text_snapshots_enabled_without_source_excerpts(
) -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-feedback-without-source.sqlite")?;
    let mut request = scan_request(
        &[chat("design-partners", 3, &["p1", "p2", "p3"])],
        &[],
        false,
        1,
        0,
    )?;
    request.feedback_text_snapshots_enabled = true;

    // When
    let result = fake_state(&db_path, native_batch()?)
        .scan_selected_chats_at(request, &db_path, &db_path)
        .map_err(|error| error.to_string())?;

    // Then
    assert_counts(&result, (1, 1, 0, 1, 0));
    let stored = scan_storage_dump(&db_path)?;
    assert!(
        stored.contains("Source excerpt hidden by settings."),
        "{stored}"
    );
    assert!(
        !stored.contains("Let's meet 2026-07-15 14:00 at the private clinic."),
        "{stored}"
    );
    Ok(())
}

#[test]
fn native_scan_rejects_malformed_feedback_text_snapshot_consent() {
    // Given
    let request = json!({
        "selectedChatIds": ["design-partners"],
        "selectedChats": [selected_chat_json("design-partners", 3, &["p1", "p2", "p3"])],
        "referenceTimezone": "Asia/Seoul",
        "referenceUnixSeconds": 1_782_352_400,
        "backfillPromptChatIds": [],
        "sourceExcerptsEnabled": true,
        "feedbackTextSnapshotsEnabled": -1,
        "listReminderProfile": list_reminder_profile::disabled_list_reminder_profile_json(),
        "localDiagnosticsEnabled": false,
        "localDiagnosticsRetentionDays": 30,
        "capPolicy": {
            "mode": "refillForPending",
            "maxVisible": 1,
            "pendingCount": 0,
        },
    });

    // When / Then
    assert!(serde_json::from_value::<ScanSelectedChatsRequest>(request).is_err());
}
