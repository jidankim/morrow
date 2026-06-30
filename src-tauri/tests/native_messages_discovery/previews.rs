use std::path::Path;

use morrow_lib::native_bridge::{
    messages_sqlite::{MessagesSqliteAdapter, MessagesSqliteLimits},
    FakeNativeBridge, MessagesDiscoveryCommandReport, MessagesPreviewRequest, NativeBridgeState,
};
use morrow_messages::MessagesDiscoveryDataSource;
use serde_json::json;

use super::support::{create_fixture, run_sqlite};

#[test]
fn loads_latest_message_previews_only_for_requested_public_chat_ids() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join("chat.db");
    create_fixture(&db_path)?;
    let adapter = MessagesSqliteAdapter::new(db_path.clone()).with_limits(MessagesSqliteLimits {
        discovery_chat_limit: 10,
        read_message_limit: 20,
    });
    let discovery = adapter
        .discover_chats()
        .map_err(|error| error.to_string())?;
    let command_discovery = MessagesDiscoveryCommandReport::from_report(&discovery);
    let discovery_json =
        serde_json::to_string(&command_discovery).map_err(|error| error.to_string())?;
    let alpha_chat_id = chat_id_for_label(&command_discovery, "Clinic Ops")?;

    // When
    let report = NativeBridgeState::default()
        .load_messages_chat_previews_at(
            &db_path,
            &MessagesPreviewRequest {
                chat_ids: vec![alpha_chat_id.clone()],
            },
        )
        .map_err(|error| error.to_string())?;
    let serialized = serde_json::to_string(&report).map_err(|error| error.to_string())?;

    // Then
    assert!(!discovery_json.contains("preview"));
    assert!(!discovery_json.contains("body"));
    assert!(!discovery_json.contains("alpha private body"));
    assert!(!discovery_json.contains("beta selected body"));
    assert_eq!(report.chats.len(), 1);
    assert_eq!(report.chats[0].chat_id, alpha_chat_id);
    assert_eq!(report.chats[0].preview, "alpha private body");
    assert!(serialized.contains("\"chatId\""));
    assert!(serialized.contains("\"preview\":\"alpha private body\""));
    assert!(!serialized.contains("beta selected body"));
    assert_preview_output_is_public_only(&serialized);
    Ok(())
}

#[test]
fn rejects_unknown_and_mixed_preview_chat_ids_without_partial_rows() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join("chat.db");
    create_fixture(&db_path)?;
    let command_discovery = discover_command_report(&db_path)?;
    let alpha_chat_id = chat_id_for_label(&command_discovery, "Clinic Ops")?;
    let state = NativeBridgeState::default();

    // When
    let unknown = state.load_messages_chat_previews_at(
        &db_path,
        &MessagesPreviewRequest {
            chat_ids: vec!["messages-chat-00000000000000000000000000000000".to_owned()],
        },
    );
    let mixed = state.load_messages_chat_previews_at(
        &db_path,
        &MessagesPreviewRequest {
            chat_ids: vec![
                alpha_chat_id,
                "messages-chat-00000000000000000000000000000000".to_owned(),
            ],
        },
    );
    let raw_guid = state.load_messages_chat_previews_at(
        &db_path,
        &MessagesPreviewRequest {
            chat_ids: vec!["iMessage;-;chat-alpha".to_owned()],
        },
    );
    let raw_handle = state.load_messages_chat_previews_at(
        &db_path,
        &MessagesPreviewRequest {
            chat_ids: vec!["alpha@example.com".to_owned()],
        },
    );

    // Then
    assert_body_free_error(unknown);
    assert_body_free_error(mixed);
    assert_body_free_error(raw_guid);
    assert_body_free_error(raw_handle);
    Ok(())
}

#[test]
fn serializes_preview_contract_with_camel_case_capped_text_and_without_raw_identifiers(
) -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join("chat.db");
    create_fixture(&db_path)?;
    run_sqlite(
        &db_path,
        &format!(
            "
            INSERT INTO message (ROWID, guid, date, text, attributedBody, handle_id)
            VALUES (4, 'alpha-long-message-guid', {}, '  alpha   private
            body with extra whitespace and enough additional scheduling details to exceed the preview character limit for this contract  ', NULL, 2);
            INSERT INTO chat_message_join (chat_id, message_id) VALUES (1, 4);
            ",
            apple_nanoseconds_for_test(1_700_000_250)
        ),
    )?;
    let command_discovery = discover_command_report(&db_path)?;
    let alpha_chat_id = chat_id_for_label(&command_discovery, "Clinic Ops")?;
    let request: MessagesPreviewRequest = serde_json::from_value(json!({
        "chatIds": [alpha_chat_id],
    }))
    .map_err(|error| error.to_string())?;

    // When
    let report = NativeBridgeState::default()
        .load_messages_chat_previews_at(&db_path, &request)
        .map_err(|error| error.to_string())?;
    let serialized = serde_json::to_string(&report).map_err(|error| error.to_string())?;

    // Then
    assert_eq!(
        report.chats[0].preview,
        "alpha private body with extra whitespace and enough additional scheduling details to exceed the preview character limit..."
    );
    assert!(serialized.contains("\"chatId\":\"messages-chat-"));
    assert!(serialized.contains("\"preview\""));
    assert!(!serialized.contains("chat_id"));
    assert!(!serialized.contains("chatGuid"));
    assert!(!serialized.contains("alpha-long-message-guid"));
    assert_preview_output_is_public_only(&serialized);
    Ok(())
}

#[test]
fn returns_body_free_errors_when_preview_source_is_denied_or_unavailable() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let request = MessagesPreviewRequest {
        chat_ids: vec!["messages-chat-00000000000000000000000000000000".to_owned()],
    };
    let denied = MessagesSqliteAdapter::permission_denied_for_test(dir.path().join("chat.db"));
    let missing_db = NativeBridgeState::default();
    let fake_unavailable = NativeBridgeState::with_bridge(FakeNativeBridge::default());

    // When
    let denied_result = denied.load_messages_chat_previews(&request);
    let missing_result =
        missing_db.load_messages_chat_previews_at(&dir.path().join("missing/chat.db"), &request);
    let fake_result =
        fake_unavailable.load_messages_chat_previews_at(Path::new("unused"), &request);

    // Then
    assert_body_free_error(denied_result);
    assert_body_free_error(missing_result);
    assert_body_free_error(fake_result);
    Ok(())
}

#[test]
fn returns_empty_preview_when_latest_message_has_no_text() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join("chat.db");
    create_fixture(&db_path)?;
    run_sqlite(
        &db_path,
        &format!(
            "
            INSERT INTO message (ROWID, guid, date, text, attributedBody, handle_id)
            VALUES (5, 'alpha-attachment-message', {}, NULL, NULL, 2);
            INSERT INTO chat_message_join (chat_id, message_id) VALUES (1, 5);
            ",
            apple_nanoseconds_for_test(1_700_000_300)
        ),
    )?;
    let command_discovery = discover_command_report(&db_path)?;
    let alpha_chat_id = chat_id_for_label(&command_discovery, "Clinic Ops")?;

    // When
    let report = NativeBridgeState::default()
        .load_messages_chat_previews_at(
            &db_path,
            &MessagesPreviewRequest {
                chat_ids: vec![alpha_chat_id],
            },
        )
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(report.chats.len(), 1);
    assert_eq!(report.chats[0].preview, "");
    Ok(())
}

fn discover_command_report(db_path: &Path) -> Result<MessagesDiscoveryCommandReport, String> {
    let report = MessagesSqliteAdapter::new(db_path.to_path_buf())
        .discover_chats()
        .map_err(|error| error.to_string())?;
    Ok(MessagesDiscoveryCommandReport::from_report(&report))
}

fn chat_id_for_label(
    report: &MessagesDiscoveryCommandReport,
    label: &str,
) -> Result<String, String> {
    report
        .chats
        .iter()
        .find(|chat| chat.display_label == label)
        .map(|chat| chat.chat_id.clone())
        .ok_or_else(|| format!("missing chat label {label}"))
}

fn assert_body_free_error<T>(result: Result<T, morrow_messages::MessagesError>) {
    let error = match result {
        Ok(_) => panic!("preview request should fail"),
        Err(error) => error,
    };
    let rendered = error.to_string();
    assert!(!rendered.contains("alpha private body"));
    assert!(!rendered.contains("beta selected body"));
    assert!(!rendered.contains("iMessage;-;"));
    assert!(!rendered.contains("+1555"));
    assert!(!rendered.contains('@'));
    assert!(!rendered.contains("message-guid"));
}

fn assert_preview_output_is_public_only(serialized: &str) {
    assert!(serialized.contains("messages-chat-"));
    assert!(!serialized.contains("iMessage;-;"));
    assert!(!serialized.contains("+1555"));
    assert!(!serialized.contains('@'));
    assert!(!serialized.contains("alpha-message"));
    assert!(!serialized.contains("raw-guid-message"));
}

const fn apple_nanoseconds_for_test(unix_seconds: i64) -> i64 {
    (unix_seconds - 978_307_200) * 1_000_000_000
}
