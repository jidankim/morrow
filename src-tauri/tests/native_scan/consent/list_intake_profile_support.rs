use morrow_lib::native_bridge::ScanSelectedChatsRequest;
use serde_json::{json, Value};

use super::super::support::selected_chat_json;

pub(super) fn valid_list_intake_profile_json() -> Value {
    json!({
        "enabled": true,
        "profileId": "list-intake-fishcount",
        "name": "Fish count",
        "profileVersion": "list-intake-v2",
        "kind": "quantityList",
        "extractionMode": "providerConstrained",
        "providerPromptVersion": "list-intake-v1",
        "positiveExamples": ["2 anchovies, 3 salmon"],
        "negativeExamples": ["remind me to buy fish tomorrow"],
        "categoryRules": [{
            "categoryId": "seafood",
            "displayName": "Seafood",
            "keywords": ["salmon"],
        }],
        "examplesHash": "list-intake-12345678",
        "aggregation": {
            "window": "localDay",
            "timezoneSource": "referenceTimezone",
        },
        "chatScope": {
            "mode": "allSelectedChats",
        },
        "grouping": {
            "chat": true,
            "sender": "off",
        },
        "captureFromScheduledMessages": false,
        "outputPolicy": "aggregateOnly",
        "quantityListBounds": {
            "minItems": 1,
            "maxItems": 20,
            "minQuantity": 1,
            "maxQuantity": 999,
            "maxItemNameVisibleChars": 80,
            "maxUnitVisibleChars": 24,
            "uncategorizedCategoryId": "uncategorized",
        },
        "thresholds": {
            "autoAggregateThresholdMillis": 850,
            "reviewThresholdMillis": 550,
        },
    })
}

pub(super) fn valid_selected_chat_scope_json() -> Value {
    json!({
        "mode": "selectedChatIds",
        "selectedChatIds": ["messages-chat-00000000000000000000000000000001"],
    })
}

pub(super) fn base_request(list_intake_profiles: Value) -> Value {
    json!({
        "selectedChatIds": ["design-partners"],
        "selectedChats": [selected_chat_json("design-partners", 3, &["p1", "p2", "p3"])],
        "referenceTimezone": "Asia/Seoul",
        "referenceUnixSeconds": 1_782_352_400,
        "backfillPromptChatIds": [],
        "sourceExcerptsEnabled": true,
        "feedbackTextSnapshotsEnabled": false,
        "localDiagnosticsEnabled": false,
        "localDiagnosticsRetentionDays": 30,
        "listIntakeProfiles": list_intake_profiles,
        "capPolicy": {
            "mode": "refillForPending",
            "maxVisible": 1,
            "pendingCount": 0,
        },
    })
}

pub(super) fn parse_request(
    list_intake_profiles: Value,
) -> serde_json::Result<ScanSelectedChatsRequest> {
    serde_json::from_value::<ScanSelectedChatsRequest>(base_request(list_intake_profiles))
}

pub(super) fn assert_rejects_profile(case: &str, profile: Value) {
    let result = parse_request(json!([profile]));
    println!("rejected_schema_case={case}");
    assert!(result.is_err(), "{case} unexpectedly parsed successfully");
}

pub(super) fn assert_rejects_profiles(case: &str, profiles: Value) {
    let result = parse_request(profiles);
    println!("rejected_schema_case={case}");
    assert!(result.is_err(), "{case} unexpectedly parsed successfully");
}
