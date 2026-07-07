use morrow_lib::native_bridge::{FakeNativeBridge, NativeBridgeState};
use morrow_messages::TapbackKind;
use morrow_storage::{CandidateState, Store};

#[path = "native_scan/consent.rs"]
mod consent;
#[path = "native_scan/contract.rs"]
mod contract;
#[path = "native_scan/dependencies.rs"]
mod dependencies;
#[path = "native_scan/feedback.rs"]
mod feedback;
#[path = "native_scan/list_reminder_profile.rs"]
mod list_reminder_profile;
#[path = "native_scan/local_diagnostics.rs"]
mod local_diagnostics;
#[path = "native_scan/message_sqlite.rs"]
mod message_sqlite;
#[path = "native_scan/outcome_plan.rs"]
mod outcome_plan;
#[path = "native_scan/persistence.rs"]
mod persistence;
#[path = "native_scan/production_trace_recorder_selection.rs"]
mod production_trace_recorder_selection;
#[path = "native_scan/proposal_replay_decision.rs"]
mod proposal_replay_decision;
#[path = "native_scan/provider_eventkit.rs"]
mod provider_eventkit;
#[path = "native_scan/provider_eventkit_calendar.rs"]
mod provider_eventkit_calendar;
#[path = "native_scan/provider_eventkit_reminders.rs"]
mod provider_eventkit_reminders;
#[path = "native_scan/provider_eventkit_task_reminders.rs"]
mod provider_eventkit_task_reminders;
#[path = "native_scan/provider_feedback.rs"]
mod provider_feedback;
#[path = "native_scan/provider_route_ledger.rs"]
mod provider_route_ledger;
#[path = "native_scan/provider_route_ledger_accounting.rs"]
mod provider_route_ledger_accounting;
#[path = "native_scan/provider_route_ledger_invalidation.rs"]
mod provider_route_ledger_invalidation;
#[path = "native_scan/provider_route_ledger_privacy.rs"]
mod provider_route_ledger_privacy;
#[path = "native_scan/provider_route_support.rs"]
mod provider_route_support;
#[path = "native_scan/replay_selection.rs"]
mod replay_selection;
#[path = "native_scan/scheduling_intent_eventkit.rs"]
mod scheduling_intent_eventkit;
#[path = "native_scan/scheduling_intent_live_receipt.rs"]
mod scheduling_intent_live_receipt;
#[path = "native_scan/scheduling_intent_provider_route.rs"]
mod scheduling_intent_provider_route;
#[path = "native_scan/scheduling_intent_support.rs"]
mod scheduling_intent_support;
#[path = "native_scan/semantic_provider_router.rs"]
mod semantic_provider_router;
#[path = "native_scan/support.rs"]
mod support;
#[path = "native_scan/trace.rs"]
mod trace;
#[path = "native_scan/trace_support.rs"]
mod trace_support;

use support::{
    assert_counts, batch, candidate_state, chat, fake_state, native_batch,
    raw_chat_with_participants, scan_request, scan_storage_dump, temp_db,
};

#[test]
fn scan_contract_preserves_counts_before_boundary_refactor() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-count-contract.sqlite")?;
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
    assert_eq!(result.created_external_proposal_count, 1);
    assert_eq!(result.failed_external_proposal_count, 0);
    assert_eq!(result.feedback_label_count, 2);
    assert_eq!(result.feature_snapshot_count, 2);
    assert_eq!(result.created_candidate_ids.len(), 1);
    Ok(())
}

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
