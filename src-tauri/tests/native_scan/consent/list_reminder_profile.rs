use morrow_lib::native_bridge::ScanSelectedChatsRequest;
use serde_json::json;

use super::super::support::selected_chat_json;
use super::list_intake_profile_support::{
    base_request, parse_request, valid_list_intake_profile_json,
};

#[test]
fn native_scan_list_intake_request_accepts_valid_profiles() -> Result<(), String> {
    // Given
    let request = base_request(json!([valid_list_intake_profile_json()]));

    // When
    let parsed = serde_json::from_value::<ScanSelectedChatsRequest>(request)
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(parsed.list_intake_profiles.len(), 1);
    assert_eq!(
        parsed.list_intake_profiles[0].profile_id,
        "list-intake-fishcount"
    );
    assert_eq!(parsed.list_intake_profiles[0].name, "Fish count");
    Ok(())
}

#[test]
fn native_scan_list_intake_request_accepts_daily_digest_profile() -> Result<(), String> {
    // Given
    let mut profile = valid_list_intake_profile_json();
    profile["outputPolicy"] = json!("dailyDigestReminder");
    profile["digestReminder"] = json!({
        "dueTimeLocal": "09:00",
        "dateOffsetDays": 1,
        "outputPolicyVersion": "list-intake-digest-v1",
    });

    // When
    let parsed = parse_request(json!([profile])).map_err(|error| error.to_string())?;

    // Then
    assert_eq!(parsed.list_intake_profiles.len(), 1);
    Ok(())
}

#[test]
fn native_scan_list_intake_request_rejects_legacy_only_list_reminder_profile() {
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
            "itemOutputMode": "singleReminderTitle",
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
fn native_scan_list_intake_request_rejects_malformed_kind() {
    // Given
    let mut profile = valid_list_intake_profile_json();
    profile["kind"] = json!("customPrompt");
    let request = base_request(json!([profile]));

    // When / Then
    assert!(serde_json::from_value::<ScanSelectedChatsRequest>(request).is_err());
}

#[test]
fn native_scan_list_intake_request_rejects_missing_profiles() {
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
