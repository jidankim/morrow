use std::cell::Cell;

use morrow_detection::{
    AiProvider, ListIntakeProviderRequest, ProviderError, ProviderRequest, ProviderResponse,
    ValidatedListIntakeExtraction,
};
use morrow_lib::native_bridge::{
    scan_selected_chats_with_dependencies, ScanSelectedChatsDependencies, ScanSelectedChatsRequest,
};
use serde_json::{json, Value};

use super::dependencies::RecordingProposalAdapter;
use super::message_sqlite::{create_messages_fixture, update_provider_route_message_text};
use super::support::query_sqlite;

const PROFILE_ID: &str = "list-intake-fishcount";

#[derive(Debug)]
pub(super) struct ListIntakeProvider {
    output: Option<String>,
    list_calls: Cell<usize>,
}

impl ListIntakeProvider {
    pub(super) fn new(output: String) -> Self {
        Self {
            output: Some(output),
            list_calls: Cell::new(0),
        }
    }

    pub(super) const fn unavailable() -> Self {
        Self {
            output: None,
            list_calls: Cell::new(0),
        }
    }

    pub(super) fn list_calls(&self) -> usize {
        self.list_calls.get()
    }
}

impl AiProvider for ListIntakeProvider {
    fn extract(&self, _request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        Ok(ProviderResponse::new(
            "{\"kind\":\"task_reminder\",\"title\":\"Scheduling fallback\",\
             \"confidence_millis\":700,\
             \"normalized_time\":\"2026-07-25T23:59:00[Asia/Seoul]\",\
             \"anchor_message_guid\":\"beta-provider-route\",\
             \"evidence_message_guids\":[\"beta-provider-route\"]}",
        ))
    }

    fn extract_list_intake(
        &self,
        request: ListIntakeProviderRequest<'_>,
    ) -> Result<ValidatedListIntakeExtraction, ProviderError> {
        self.list_calls.set(self.list_calls.get() + 1);
        let Some(output) = &self.output else {
            return Err(ProviderError::Unavailable {
                reason: "test list-intake provider unavailable".to_owned(),
            });
        };
        morrow_detection::validate_list_intake_provider_output(
            request.profile(),
            request.evidence(),
            request.reference_timezone(),
            output,
        )
        .map_err(|error| ProviderError::Unavailable {
            reason: error.to_string(),
        })
    }
}

pub(super) struct ListIntakeFixture {
    _dir: tempfile::TempDir,
    pub(super) store_path: std::path::PathBuf,
    messages_db_path: std::path::PathBuf,
}

impl ListIntakeFixture {
    pub(super) fn with_text(text: &str) -> Result<Self, String> {
        let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let store_path = dir.path().join("morrow.sqlite");
        let messages_db_path = dir.path().join("chat.db");
        create_messages_fixture(&messages_db_path)?;
        update_provider_route_message_text(&messages_db_path, text)?;
        Ok(Self {
            _dir: dir,
            store_path,
            messages_db_path,
        })
    }

    pub(super) fn scan<P: AiProvider>(
        &self,
        request: ScanSelectedChatsRequest,
        provider: &P,
    ) -> Result<morrow_lib::native_bridge::ScanSelectedChatsResult, String> {
        let source = morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter::new(
            self.messages_db_path.clone(),
        );
        let adapter = RecordingProposalAdapter::default();
        let recorder = morrow_diagnostics::NoopTraceRecorder;
        scan_selected_chats_with_dependencies(
            request,
            &self.store_path,
            ScanSelectedChatsDependencies {
                source: &source,
                provider,
                proposal_adapter: &adapter,
                trace_recorder: &recorder,
            },
        )
        .map_err(|error| error.to_string())
    }
}

pub(super) fn request_with_profiles(
    profiles: Vec<Value>,
) -> Result<ScanSelectedChatsRequest, String> {
    serde_json::from_value(json!({
        "selectedChatIds": ["iMessage;-;+15555550103"],
        "selectedChats": [{
            "id": "iMessage;-;+15555550103",
            "participantCount": 1,
            "participantIds": ["messages-participant-6044b729eea9fa126e78d421e4a41ac8"]
        }],
        "referenceTimezone": "Asia/Seoul",
        "referenceUnixSeconds": 1_782_352_400,
        "backfillPromptChatIds": [],
        "sourceExcerptsEnabled": true,
        "feedbackTextSnapshotsEnabled": false,
        "localDiagnosticsEnabled": false,
        "localDiagnosticsRetentionDays": 30,
        "listIntakeProfiles": profiles,
        "capPolicy": {
            "mode": "refillForPending",
            "maxVisible": 1,
            "pendingCount": 0
        }
    }))
    .map_err(|error| error.to_string())
}

pub(super) fn profile_json(
    enabled: bool,
    capture_from_scheduled_messages: bool,
    chat_scope: Value,
) -> Value {
    json!({
        "enabled": enabled,
        "profileId": PROFILE_ID,
        "name": "Fish count",
        "profileVersion": "list-intake-v2",
        "kind": "quantityList",
        "extractionMode": "providerConstrained",
        "providerPromptVersion": "list-intake-v1",
        "positiveExamples": ["2 salmon, 3 tuna"],
        "negativeExamples": ["meet tomorrow at 3"],
        "categoryRules": [{ "categoryId": "fish", "displayName": "Fish", "keywords": ["salmon", "tuna"] }],
        "examplesHash": "examplesHashNativeT7",
        "aggregation": { "window": "localDay", "timezoneSource": "referenceTimezone" },
        "chatScope": chat_scope,
        "grouping": { "chat": true, "sender": "displayAlias" },
        "captureFromScheduledMessages": capture_from_scheduled_messages,
        "outputPolicy": "aggregateOnly",
        "digestReminder": null,
        "quantityListBounds": {
            "minItems": 1, "maxItems": 20, "minQuantity": 1, "maxQuantity": 999,
            "maxItemNameVisibleChars": 80, "maxUnitVisibleChars": 24,
            "uncategorizedCategoryId": "uncategorized"
        },
        "thresholds": { "autoAggregateThresholdMillis": 850, "reviewThresholdMillis": 550 },
        "migrationState": null
    })
}

pub(super) fn chat_scope_all() -> Value {
    json!({ "mode": "allSelectedChats" })
}

pub(super) fn chat_scope_selected(chat_id: &str) -> Value {
    json!({ "mode": "selectedChatIds", "selectedChatIds": [chat_id] })
}

pub(super) fn provider_json(confidence_millis: u16) -> String {
    json!({
        "matched": true,
        "confidence_millis": confidence_millis,
        "items": [
            { "name": "salmon", "quantity": 2, "categoryId": "fish", "evidenceText": "2 salmon" },
            { "name": "tuna", "quantity": 3, "categoryId": "fish", "evidenceText": "3 tuna" }
        ]
    })
    .to_string()
}

pub(super) fn provider_json_salmon(confidence_millis: u16) -> String {
    json!({
        "matched": true,
        "confidence_millis": confidence_millis,
        "items": [
            { "name": "salmon", "quantity": 2, "categoryId": "fish", "evidenceText": "2 salmon" }
        ]
    })
    .to_string()
}

pub(super) fn list_intake_entry_count(db_path: &std::path::Path) -> Result<i64, String> {
    sqlite_count(db_path, "list_intake_entries")
}

pub(super) fn list_intake_proposal_count(db_path: &std::path::Path) -> Result<i64, String> {
    sqlite_count(db_path, "list_intake_proposals")
}

pub(super) fn list_intake_proposal_item_count(db_path: &std::path::Path) -> Result<i64, String> {
    sqlite_count(db_path, "list_intake_proposal_items")
}

pub(super) fn list_intake_aggregate_count(db_path: &std::path::Path) -> Result<i64, String> {
    query_sqlite(
        db_path,
        "SELECT COUNT(*) FROM (
            SELECT profile_id, window_local_date, chat_key, sender_label, category_id, item_name, unit
            FROM list_intake_entries
            GROUP BY profile_id, window_local_date, chat_key, sender_label, category_id, item_name, unit
        );",
    )?
    .trim()
    .parse::<i64>()
    .map_err(|error| error.to_string())
}

fn sqlite_count(db_path: &std::path::Path, table: &str) -> Result<i64, String> {
    query_sqlite(db_path, &format!("SELECT COUNT(*) FROM {table};"))?
        .trim()
        .parse::<i64>()
        .map_err(|error| error.to_string())
}
