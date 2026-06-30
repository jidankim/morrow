use morrow_lib::native_bridge::{FakeNativeBridge, NativeBridgeState};
use morrow_messages::{NativeBatch, TapbackKind};
use morrow_storage::{CandidateState, Store};
use serde_json::json;

#[path = "native_scan/consent.rs"]
mod consent;
#[path = "native_scan/dependencies.rs"]
mod dependencies;
#[path = "native_scan/feedback.rs"]
mod feedback;
#[path = "native_scan/message_sqlite.rs"]
mod message_sqlite;
#[path = "native_scan/provider_eventkit.rs"]
mod provider_eventkit;
#[path = "native_scan/provider_feedback.rs"]
mod provider_feedback;
#[path = "native_scan/support.rs"]
mod support;
#[path = "native_scan/trace.rs"]
mod trace;
#[path = "native_scan/trace_support.rs"]
mod trace_support;

use support::{
    assert_counts, batch, candidate_state, chat, fake_state, malformed_request, native_batch,
    raw_chat_with_participants, scan_request, scan_storage_dump, selected_chat_json, temp_db,
};

#[test]
fn scan_selected_chats_redacts_native_anchors_and_excerpts_in_storage() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-redaction.sqlite")?;
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
    let stored = scan_storage_dump(&db_path)?;
    assert!(stored.contains("messages-chat-"), "{stored}");
    assert!(stored.contains("messages-message-"), "{stored}");
    assert!(
        stored.contains("Source excerpt hidden by settings."),
        "{stored}"
    );
    for forbidden in [
        "design-partners",
        "ops-triage",
        "msg-design",
        "msg-ops",
        "Let's meet 2026-07-15 14:00 at the private clinic.",
        "The private clinic status update was funny.",
    ] {
        assert!(
            !stored.contains(forbidden),
            "stored native scan data leaked {forbidden}: {stored}"
        );
    }
    Ok(())
}

#[test]
fn scan_selected_chats_refill_policy_defers_when_pending_fills_cap() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-refill.sqlite")?;
    let request = scan_request(
        &[chat("design-partners", 3, &["p1", "p2", "p3"])],
        &[],
        true,
        1,
        1,
    )?;
    // When
    let result = fake_state(&db_path, native_batch()?)
        .scan_selected_chats_at(request, &db_path, &db_path)
        .map_err(|error| error.to_string())?;
    // Then
    assert_counts(&result, (1, 1, 0, 0, 1));
    let store = Store::open(&db_path).map_err(|error| error.to_string())?;
    assert_eq!(candidate_state(&store, &result)?, CandidateState::Queued);
    Ok(())
}

#[test]
fn scan_selected_chats_records_degraded_message_boundary_without_candidates() -> Result<(), String>
{
    // Given
    let (_dir, db_path) = temp_db("native-scan-unavailable.sqlite")?;
    let state =
        NativeBridgeState::with_bridge(FakeNativeBridge::with_morrow_store_path(db_path.clone()));
    let request = scan_request(
        &[chat("design-partners", 3, &["p1", "p2", "p3"])],
        &[],
        true,
        1,
        0,
    )?;
    // When
    let result = state
        .scan_selected_chats_at(request, &db_path, &db_path)
        .map_err(|error| error.to_string())?;
    // Then
    assert_counts(&result, (0, 0, 0, 0, 0));
    Ok(())
}

#[test]
fn scan_selected_chats_honors_selected_participant_metadata() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-participants.sqlite")?;
    let messages = batch(vec![raw_chat_with_participants(
        "two-person-design",
        "msg-two-person",
        2,
        &["local-a", "local-b"],
        "Let's meet 2026-07-15 14:00 at the private clinic.",
        Some(TapbackKind::Like),
    )?]);
    let request = scan_request(
        &[chat("two-person-design", 2, &["local-a", "local-b"])],
        &[],
        true,
        1,
        0,
    )?;
    // When
    let result = fake_state(&db_path, messages)
        .scan_selected_chats_at(request, &db_path, &db_path)
        .map_err(|error| error.to_string())?;
    // Then
    assert_counts(&result, (1, 1, 0, 1, 0));
    Ok(())
}

#[test]
fn scan_selected_chats_pauses_when_same_count_participant_ids_change() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-participant-swap.sqlite")?;
    let messages = batch(vec![raw_chat_with_participants(
        "design-partners",
        "msg-design",
        3,
        &["p1", "p2", "p4"],
        "Let's meet 2026-07-15 14:00 at the private clinic.",
        Some(TapbackKind::Like),
    )?]);
    let request = scan_request(
        &[chat("design-partners", 3, &["p1", "p2", "p3"])],
        &[],
        true,
        1,
        0,
    )?;
    // When
    let result = fake_state(&db_path, messages)
        .scan_selected_chats_at(request, &db_path, &db_path)
        .map_err(|error| error.to_string())?;
    // Then
    assert_counts(&result, (0, 0, 0, 0, 0));
    Ok(())
}

#[test]
fn scan_selected_chats_rejects_malformed_selected_metadata() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-malformed.sqlite")?;
    let state = fake_state(&db_path, NativeBatch::default());
    let cases = [
        (
            "empty selected list",
            json!([]),
            json!([]),
            "selected_chats",
        ),
        (
            "mismatched selected ids",
            json!(["chat-a"]),
            json!([selected_chat_json("chat-b", 1, &["p1"])]),
            "selected_chats",
        ),
        (
            "duplicate participant ids",
            json!(["chat-a"]),
            json!([selected_chat_json("chat-a", 2, &["p1", "p1"])]),
            "participant_ids",
        ),
        (
            "invalid guid",
            json!([""]),
            json!([selected_chat_json("", 1, &["p1"])]),
            "chat_guid",
        ),
    ];
    for (name, selected_chat_ids, selected_chats, expected) in cases {
        // When
        let error = state
            .scan_selected_chats_at(
                malformed_request(selected_chat_ids, selected_chats)?,
                &db_path,
                &db_path,
            )
            .err()
            .ok_or_else(|| format!("{name}: scan unexpectedly succeeded"))?
            .to_string();
        // Then
        assert!(error.contains(expected), "{name}: {error}");
    }
    Ok(())
}

#[test]
fn scan_selected_chats_rejects_unsupported_reference_timezone() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-timezone.sqlite")?;
    let state = fake_state(&db_path, native_batch()?);
    let mut request = scan_request(
        &[chat("design-partners", 3, &["p1", "p2", "p3"])],
        &[],
        true,
        1,
        0,
    )?;
    request.reference_timezone = "America/Los_Angeles".to_owned();

    // When
    let error = state
        .scan_selected_chats_at(request, &db_path, &db_path)
        .err()
        .ok_or_else(|| "scan unexpectedly succeeded".to_owned())?
        .to_string();

    // Then
    assert!(error.contains("unsupported reference timezone"), "{error}");
    assert!(!error.contains("America/Los_Angeles"), "{error}");
    Ok(())
}
