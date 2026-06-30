use std::path::Path;

use morrow_lib::native_bridge::{
    messages_sqlite::{MessagesSqliteAdapter, MessagesSqliteLimits},
    FakeNativeBridge, MessagesDiscoveryCommandReport, NativeBridgeState,
};
use morrow_messages::{
    ChatGuid, MessageTimestamp, MessagesDataSource, MessagesDiscoveryDataSource,
    MessagesDiscoveryReport, MessagesDiscoveryStatus, NativeReadRequest,
};

#[path = "native_messages_discovery/attributed_body.rs"]
mod attributed_body;
#[path = "native_messages_discovery/previews.rs"]
mod previews;
#[path = "native_messages_discovery/support.rs"]
mod support;

use support::{create_fixture, run_sqlite};

#[test]
fn discovers_recent_chats_as_sorted_metadata_when_fixture_contains_raw_handles(
) -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join("chat.db");
    create_fixture(&db_path)?;
    let adapter = MessagesSqliteAdapter::new(db_path).with_limits(MessagesSqliteLimits {
        discovery_chat_limit: 10,
        read_message_limit: 20,
    });

    // When
    let report = adapter
        .discover_chats()
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(report.status(), MessagesDiscoveryStatus::Ready);
    let chats = report.chats();
    assert_eq!(chats.len(), 2);
    assert_eq!(chats[0].chat_guid().as_str(), "iMessage;-;+15555550103");
    assert_eq!(chats[1].chat_guid().as_str(), "iMessage;-;chat-alpha");
    assert_eq!(chats[0].display_label(), "Messages chat");
    assert_eq!(chats[0].participant_count(), 1);
    assert_eq!(chats[1].participant_count(), 2);
    assert_eq!(chats[0].latest_activity_timestamp().as_i64(), 1_700_000_200);
    for chat in chats {
        for participant in chat.participant_ids() {
            let value = participant.as_str();
            assert!(value.starts_with("messages-participant-"));
            assert_eq!(value.len(), "messages-participant-".len() + 32);
            assert!(!value.contains('@'));
            assert!(!value.contains("+1555"));
            assert!(!value.ends_with("-1"));
            assert!(!value.ends_with("-2"));
            assert!(!value.ends_with("-3"));
        }
    }
    Ok(())
}

#[test]
fn discovery_command_serializes_opaque_chat_ids_when_raw_guid_is_handle_shaped(
) -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join("chat.db");
    create_fixture(&db_path)?;
    let report = MessagesSqliteAdapter::new(db_path)
        .discover_chats()
        .map_err(|error| error.to_string())?;

    // When
    let command_report = MessagesDiscoveryCommandReport::from_report(&report);
    let serialized = serde_json::to_string(&command_report).map_err(|error| error.to_string())?;

    // Then
    let chat_id = &command_report.chats[0].chat_id;
    assert_eq!(chat_id, "messages-chat-8b96e568c027a42d3b5c9e6e7710201f");
    assert!(serialized.contains("\"displayLabel\":\"Messages chat\""));
    assert!(!serialized.contains("display_label"));
    assert!(!serialized.contains("latestMessageBody"));
    assert!(!serialized.contains("messagePreview"));
    assert!(!serialized.contains("private body"));
    assert!(!serialized.contains("selected body"));
    assert!(!serialized.contains("chatGuid"));
    assert!(!serialized.contains("iMessage;-;+15555550103"));
    assert!(!serialized.contains("+15555550103"));
    Ok(())
}

#[test]
fn reads_recent_messages_only_for_selected_chat_guids() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join("chat.db");
    create_fixture(&db_path)?;
    let adapter = MessagesSqliteAdapter::new(db_path);
    let request = NativeReadRequest {
        chat_guids: vec![
            ChatGuid::parse("iMessage;-;+15555550103").map_err(|error| error.to_string())?
        ],
        since: MessageTimestamp::new(1_700_000_000).map_err(|error| error.to_string())?,
        until: MessageTimestamp::new(1_700_000_300).map_err(|error| error.to_string())?,
    };

    // When
    let batch = adapter
        .read_recent(&request)
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(batch.chats.len(), 1);
    let chat = &batch.chats[0];
    assert_eq!(chat.guid.as_str(), "iMessage;-;+15555550103");
    assert_eq!(chat.participant_count, 1);
    assert_eq!(
        chat.participant_ids[0].as_str(),
        "messages-participant-6044b729eea9fa126e78d421e4a41ac8"
    );
    assert_eq!(chat.messages.len(), 1);
    assert_eq!(
        chat.messages[0].chat_guid.as_str(),
        "iMessage;-;+15555550103"
    );
    assert_eq!(chat.messages[0].message_guid.as_str(), "raw-guid-message");
    assert_eq!(chat.messages[0].timestamp.as_i64(), 1_700_000_200);
    assert_eq!(chat.messages[0].text, "beta selected body");
    Ok(())
}

#[test]
fn degrades_cleanly_when_messages_database_is_unavailable() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let missing_path = dir.path().join("missing").join("chat.db");
    let adapter = MessagesSqliteAdapter::new(missing_path);
    let incompatible_path = dir.path().join("incompatible-chat.db");
    run_sqlite(&incompatible_path, "CREATE TABLE unrelated (id INTEGER);")?;
    let incompatible_adapter = MessagesSqliteAdapter::new(incompatible_path);

    // When
    let report = adapter
        .discover_chats()
        .map_err(|error| error.to_string())?;
    let incompatible_report = incompatible_adapter
        .discover_chats()
        .map_err(|error| error.to_string())?;
    let batch = adapter
        .read_recent(&NativeReadRequest {
            chat_guids: Vec::new(),
            since: MessageTimestamp::new(1).map_err(|error| error.to_string())?,
            until: MessageTimestamp::new(2).map_err(|error| error.to_string())?,
        })
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(report.status(), MessagesDiscoveryStatus::Unavailable);
    assert!(report.chats().is_empty());
    assert_eq!(
        incompatible_report.status(),
        MessagesDiscoveryStatus::Unavailable
    );
    assert!(batch.chats.is_empty());
    Ok(())
}

#[test]
fn maps_permission_denied_sqlite_failure_to_discovery_status() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let adapter = MessagesSqliteAdapter::permission_denied_for_test(dir.path().join("chat.db"));

    // When
    let report = adapter
        .discover_chats()
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(report.status(), MessagesDiscoveryStatus::PermissionDenied);
    assert!(report.chats().is_empty());
    Ok(())
}

#[test]
fn discover_messages_chats_handles_degraded_and_production_paths() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join("chat.db");
    let missing_path = dir.path().join("private-chat.db");
    create_fixture(&db_path)?;
    let production = NativeBridgeState::default();
    let denied = NativeBridgeState::with_bridge(
        FakeNativeBridge::default()
            .with_discovery_report(MessagesDiscoveryReport::permission_denied()),
    );
    let unavailable = NativeBridgeState::with_bridge(
        FakeNativeBridge::default().with_discovery_report(MessagesDiscoveryReport::unavailable()),
    );

    // When
    let report = production
        .discover_messages_chats_at(&db_path)
        .map_err(|error| error.to_string())?;
    let missing = production
        .discover_messages_chats_at(&missing_path)
        .map_err(|error| error.to_string())?;
    let denied_report = denied
        .discover_messages_chats_at(Path::new("/private/var/denied/chat.db"))
        .map_err(|error| error.to_string())?;
    let unavailable_report = unavailable
        .discover_messages_chats_at(Path::new("/private/var/unavailable/chat.db"))
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(report.status(), MessagesDiscoveryStatus::Ready);
    assert_eq!(report.chats().len(), 2);
    assert_eq!(
        report.chats()[0].chat_guid().as_str(),
        "iMessage;-;+15555550103"
    );
    assert_eq!(
        report.chats()[1].chat_guid().as_str(),
        "iMessage;-;chat-alpha"
    );
    assert_eq!(missing.status(), MessagesDiscoveryStatus::Unavailable);
    assert!(!format!("{missing:#?}").contains(&missing_path.display().to_string()));
    assert_eq!(
        denied_report.status(),
        MessagesDiscoveryStatus::PermissionDenied
    );
    assert!(denied_report.chats().is_empty());
    assert_eq!(
        unavailable_report.status(),
        MessagesDiscoveryStatus::Unavailable
    );
    assert!(unavailable_report.chats().is_empty());
    Ok(())
}
