use morrow_lib::native_bridge::{
    scan_selected_chats_with_dependencies, ScanSelectedChatsDependencies, ScanSelectedChatsRequest,
};
use serde_json::json;

use super::dependencies::RecordingProposalAdapter;
use super::message_sqlite::create_messages_fixture;
use super::support::{chat, scan_request};

pub(super) struct ProviderRouteFixture {
    pub(super) _dir: tempfile::TempDir,
    pub(super) store_path: std::path::PathBuf,
    pub(super) messages_db_path: std::path::PathBuf,
    pub(super) source: morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter,
}

impl ProviderRouteFixture {
    pub(super) fn new() -> Result<Self, String> {
        let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let store_path = dir.path().join("morrow.sqlite");
        let messages_db_path = dir.path().join("chat.db");
        create_messages_fixture(&messages_db_path)?;
        Ok(Self {
            source: morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter::new(
                messages_db_path.clone(),
            ),
            _dir: dir,
            store_path,
            messages_db_path,
        })
    }
}

pub(super) fn provider_route_request() -> Result<ScanSelectedChatsRequest, String> {
    scan_request(
        &[chat(
            "iMessage;-;+15555550103",
            1,
            &["messages-participant-6044b729eea9fa126e78d421e4a41ac8"],
        )],
        &[],
        true,
        1,
        0,
    )
}

pub(super) fn provider_route_request_with_options(
    source_excerpts_enabled: bool,
    reference_timezone: &str,
    reference_unix_seconds: i64,
) -> Result<ScanSelectedChatsRequest, String> {
    serde_json::from_value(json!({
        "selectedChatIds": ["iMessage;-;+15555550103"],
        "selectedChats": [{
            "id": "iMessage;-;+15555550103",
            "participantCount": 1,
            "participantIds": ["messages-participant-6044b729eea9fa126e78d421e4a41ac8"],
        }],
        "referenceTimezone": reference_timezone,
        "referenceUnixSeconds": reference_unix_seconds,
        "backfillPromptChatIds": [],
        "sourceExcerptsEnabled": source_excerpts_enabled,
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
    }))
    .map_err(|error| error.to_string())
}

pub(super) fn scan_provider_route<P, R>(
    fixture: &ProviderRouteFixture,
    request: ScanSelectedChatsRequest,
    provider: &P,
    adapter: &RecordingProposalAdapter,
    recorder: &R,
) -> Result<morrow_lib::native_bridge::ScanSelectedChatsResult, String>
where
    P: morrow_detection::AiProvider,
    R: morrow_diagnostics::TraceRecorder + ?Sized,
{
    scan_selected_chats_with_dependencies(
        request,
        &fixture.store_path,
        ScanSelectedChatsDependencies {
            source: &fixture.source,
            provider,
            proposal_adapter: adapter,
            trace_recorder: recorder,
        },
    )
    .map_err(|error| error.to_string())
}

pub(super) fn assert_provider_route_dump_hides(dump: &str, forbidden: &[&str]) {
    for value in forbidden {
        assert!(
            !dump.contains(value),
            "provider route ledger leaked forbidden substring {value}: {dump}"
        );
    }
}
