use morrow_lib::native_bridge::{ListReminderRoutingMode, ScanSelectedChatsRequest};
use serde_json::{json, Value};

use super::super::support::selected_chat_json;

pub(super) fn disabled_list_reminder_profile_json() -> Value {
    json!({
        "enabled": false,
        "profileId": "list-reminders",
        "profileVersion": "list-reminders-v1",
        "routingMode": "explicitOnly",
        "defaultDueMode": "explicitOnly",
        "defaultDueTime": "23:59",
        "recurrenceMode": "none",
        "itemOutputMode": "singleReminderTitle",
    })
}

#[test]
fn native_scan_accepts_valid_disabled_list_reminder_profile() -> Result<(), String> {
    // Given
    let request = json!({
        "selectedChatIds": ["design-partners"],
        "selectedChats": [selected_chat_json("design-partners", 3, &["p1", "p2", "p3"])],
        "referenceTimezone": "Asia/Seoul",
        "referenceUnixSeconds": 1_782_352_400,
        "backfillPromptChatIds": [],
        "sourceExcerptsEnabled": true,
        "feedbackTextSnapshotsEnabled": false,
        "localDiagnosticsEnabled": false,
        "localDiagnosticsRetentionDays": 30,
        "listReminderProfile": disabled_list_reminder_profile_json(),
        "capPolicy": {
            "mode": "refillForPending",
            "maxVisible": 1,
            "pendingCount": 0,
        },
    });

    // When
    let parsed = serde_json::from_value::<ScanSelectedChatsRequest>(request)
        .map_err(|error| error.to_string())?;

    // Then
    assert!(!parsed.list_reminder_profile.enabled);
    assert_eq!(
        parsed.list_reminder_profile.profile_id.as_str(),
        "list-reminders"
    );
    assert_eq!(
        parsed.list_reminder_profile.profile_version.as_str(),
        "list-reminders-v1"
    );
    assert_eq!(
        parsed.list_reminder_profile.routing_mode,
        ListReminderRoutingMode::ExplicitOnly
    );
    Ok(())
}

#[test]
fn native_scan_rejects_missing_list_reminder_profile() {
    // Given
    let request = json!({
        "selectedChatIds": ["design-partners"],
        "selectedChats": [selected_chat_json("design-partners", 3, &["p1", "p2", "p3"])],
        "referenceTimezone": "Asia/Seoul",
        "referenceUnixSeconds": 1_782_352_400,
        "backfillPromptChatIds": [],
        "sourceExcerptsEnabled": true,
        "feedbackTextSnapshotsEnabled": false,
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
fn native_scan_rejects_malformed_list_reminder_profile() {
    // Given
    let request = json!({
        "selectedChatIds": ["design-partners"],
        "selectedChats": [selected_chat_json("design-partners", 3, &["p1", "p2", "p3"])],
        "referenceTimezone": "Asia/Seoul",
        "referenceUnixSeconds": 1_782_352_400,
        "backfillPromptChatIds": [],
        "sourceExcerptsEnabled": true,
        "feedbackTextSnapshotsEnabled": false,
        "localDiagnosticsEnabled": false,
        "localDiagnosticsRetentionDays": 30,
        "listReminderProfile": {
            "enabled": true,
            "profileId": "list-reminders",
            "profileVersion": "list-reminders-v1",
            "routingMode": "routeEverything",
            "defaultDueMode": "tomorrowish",
            "defaultDueTime": "24:00",
            "recurrenceMode": "daily",
            "itemOutputMode": "perItemReminder",
        },
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
fn native_scan_rejects_incomplete_list_reminder_profile() {
    // Given
    let request = json!({
        "selectedChatIds": ["design-partners"],
        "selectedChats": [selected_chat_json("design-partners", 3, &["p1", "p2", "p3"])],
        "referenceTimezone": "Asia/Seoul",
        "referenceUnixSeconds": 1_782_352_400,
        "backfillPromptChatIds": [],
        "sourceExcerptsEnabled": true,
        "feedbackTextSnapshotsEnabled": false,
        "localDiagnosticsEnabled": false,
        "localDiagnosticsRetentionDays": 30,
        "listReminderProfile": {
            "enabled": false,
            "profileId": "list-reminders",
            "profileVersion": "list-reminders-v1",
            "routingMode": "explicitOnly",
            "defaultDueMode": "explicitOnly",
            "defaultDueTime": "23:59",
            "recurrenceMode": "none",
        },
        "capPolicy": {
            "mode": "refillForPending",
            "maxVisible": 1,
            "pendingCount": 0,
        },
    });

    // When / Then
    assert!(serde_json::from_value::<ScanSelectedChatsRequest>(request).is_err());
}
