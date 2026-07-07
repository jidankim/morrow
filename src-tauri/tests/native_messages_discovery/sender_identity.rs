use std::collections::BTreeMap;
use std::path::Path;

use morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter;
use morrow_messages::{
    ingest_selected_threads, BackfillDays, ChatGuid, IngestionReport, IngestionRequest,
    MessageTimestamp, MessagesDataSource, NativeReadRequest, WhitelistedChat,
};
use morrow_storage::Store;
use sha2::{Digest, Sha256};

use super::support::run_sqlite;

#[test]
fn sender_identity_extracts_display_aliases_and_hides_private_keys() -> Result<(), String> {
    // Given: selected Messages rows include two handles, null, empty, and invalid sender handles.
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join("sender-chat.db");
    let store_path = dir.path().join("morrow.sqlite");
    create_sender_fixture(&db_path)?;
    let store = Store::open(&store_path).map_err(|error| error.to_string())?;
    let adapter = MessagesSqliteAdapter::with_local_store(db_path.clone(), &store)
        .map_err(|error| error.to_string())?;
    let chat_guid =
        ChatGuid::parse("iMessage;-;chat-sender-fixture").map_err(|error| error.to_string())?;
    let read_request = NativeReadRequest {
        chat_guids: vec![chat_guid.clone()],
        since: MessageTimestamp::new(1_700_000_000).map_err(|error| error.to_string())?,
        until: MessageTimestamp::new(1_700_000_300).map_err(|error| error.to_string())?,
    };
    let ingestion_request = IngestionRequest::new(
        vec![WhitelistedChat::new(chat_guid, 2).map_err(|error| error.to_string())?],
        BackfillDays::new(7).map_err(|error| error.to_string())?,
        MessageTimestamp::new(1_700_000_300).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;

    // When: native ingestion reads the sender projection and ingestion derives safe labels twice.
    let batch = adapter
        .read_recent(&read_request)
        .map_err(|error| error.to_string())?;
    let first =
        ingest_selected_threads(&adapter, &ingestion_request).map_err(|error| error.to_string())?;
    let second_store = Store::open(&store_path).map_err(|error| error.to_string())?;
    let second_adapter = MessagesSqliteAdapter::with_local_store(db_path, &second_store)
        .map_err(|error| error.to_string())?;
    let second = ingest_selected_threads(&second_adapter, &ingestion_request)
        .map_err(|error| error.to_string())?;

    // Then: labels are stable, null/unusable senders share one label, and private tokens stay hidden.
    assert_eq!(batch.chats.len(), 1);
    assert_eq!(batch.chats[0].messages.len(), 6);
    let first_labels = labels_by_message(&first);
    let second_labels = labels_by_message(&second);
    assert_eq!(first_labels, second_labels);
    assert_eq!(
        first_labels.get("msg-phone"),
        first_labels.get("msg-phone-again")
    );
    assert_ne!(first_labels.get("msg-phone"), first_labels.get("msg-email"));
    assert_eq!(
        first_labels.get("msg-null"),
        Some(&"Unknown sender".to_owned())
    );
    assert_eq!(first_labels.get("msg-empty"), first_labels.get("msg-null"));
    assert_eq!(
        first_labels.get("msg-invalid"),
        first_labels.get("msg-null")
    );
    let report_debug = format!("{first:?}");
    assert_private_sender_tokens_absent(&report_debug);
    println!(
        "sender_identity labels={} unknown_bucket=stable privacy=hidden",
        first_labels.len()
    );
    Ok(())
}

#[test]
fn sender_identity_salt_is_persisted_random_and_store_scoped() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join("sender-chat.db");
    let first_store_path = dir.path().join("first-morrow.sqlite");
    let second_store_path = dir.path().join("second-morrow.sqlite");
    create_sender_fixture(&db_path)?;

    let first_store = Store::open(&first_store_path).map_err(|error| error.to_string())?;
    let first_adapter = MessagesSqliteAdapter::with_local_store(db_path.clone(), &first_store)
        .map_err(|error| error.to_string())?;
    let first_adapter_debug = format!("{first_adapter:?}");
    let first_salt = persisted_sender_salt(&first_store_path)?;
    assert_sender_salt_debug_redacted(&first_adapter_debug, &first_salt)?;

    let reopened_store = Store::open(&first_store_path).map_err(|error| error.to_string())?;
    let _reopened_adapter =
        MessagesSqliteAdapter::with_local_store(db_path.clone(), &reopened_store)
            .map_err(|error| error.to_string())?;
    let reopened_salt = persisted_sender_salt(&first_store_path)?;

    let second_store = Store::open(&second_store_path).map_err(|error| error.to_string())?;
    let _second_adapter = MessagesSqliteAdapter::with_local_store(db_path.clone(), &second_store)
        .map_err(|error| error.to_string())?;
    let second_salt = persisted_sender_salt(&second_store_path)?;
    let legacy_path_salt = legacy_messages_path_salt_hex(&db_path);

    assert!(
        first_salt == reopened_salt,
        "sender salt did not persist for the same Morrow DB"
    );
    assert!(
        first_salt != second_salt,
        "independent Morrow DBs reused the same sender salt"
    );
    assert!(
        sender_salt_hex(&first_salt)? != legacy_path_salt,
        "sender salt matched the legacy Messages-path derivation"
    );
    println!(
        "sender_identity_salt stable_same_store=true independent_store=true legacy_path_derivation=false"
    );
    Ok(())
}

fn create_sender_fixture(db_path: &Path) -> Result<(), String> {
    let sql = format!(
        "
        CREATE TABLE chat (ROWID INTEGER PRIMARY KEY, guid TEXT NOT NULL, display_name TEXT);
        CREATE TABLE handle (ROWID INTEGER PRIMARY KEY, id TEXT NOT NULL);
        CREATE TABLE message (
            ROWID INTEGER PRIMARY KEY,
            guid TEXT NOT NULL,
            date INTEGER NOT NULL,
            text TEXT,
            attributedBody BLOB,
            handle_id INTEGER
        );
        CREATE TABLE chat_message_join (chat_id INTEGER NOT NULL, message_id INTEGER NOT NULL);
        CREATE TABLE chat_handle_join (chat_id INTEGER NOT NULL, handle_id INTEGER NOT NULL);
        INSERT INTO chat (ROWID, guid, display_name)
            VALUES (1, 'iMessage;-;chat-sender-fixture', 'Sender fixture');
        INSERT INTO handle (ROWID, id)
            VALUES (1, '+15555550103'), (2, 'ALPHA@example.COM'), (3, ''), (4, char(0));
        INSERT INTO chat_handle_join (chat_id, handle_id) VALUES (1, 1), (1, 2);
        INSERT INTO message (ROWID, guid, date, text, attributedBody, handle_id)
            VALUES (1, 'msg-phone', {}, '1 apples', NULL, 1),
                   (2, 'msg-email', {}, '2 bananas', NULL, 2),
                   (3, 'msg-null', {}, '3 carrots', NULL, NULL),
                   (4, 'msg-empty', {}, '4 dates', NULL, 3),
                   (5, 'msg-invalid', {}, '5 eggplant', NULL, 4),
                   (6, 'msg-phone-again', {}, '6 figs', NULL, 1);
        INSERT INTO chat_message_join (chat_id, message_id)
            VALUES (1, 1), (1, 2), (1, 3), (1, 4), (1, 5), (1, 6);
        ",
        apple_nanoseconds(1_700_000_100),
        apple_nanoseconds(1_700_000_101),
        apple_nanoseconds(1_700_000_102),
        apple_nanoseconds(1_700_000_103),
        apple_nanoseconds(1_700_000_104),
        apple_nanoseconds(1_700_000_105),
    );
    run_sqlite(db_path, &sql)
}

fn persisted_sender_salt(db_path: &Path) -> Result<String, String> {
    query_sqlite_first(
        db_path,
        "SELECT value_json FROM settings WHERE key = 'list_intake_sender_salt_v1';",
    )
}

fn query_sqlite_first(db_path: &Path, sql: &str) -> Result<String, String> {
    use std::process::{Command, Stdio};
    let mut child = Command::new("sqlite3")
        .arg(db_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "failed to open sqlite stdin".to_owned())?;
    use std::io::Write;
    stdin
        .write_all(sql.as_bytes())
        .map_err(|error| error.to_string())?;
    drop(stdin);
    let output = child
        .wait_with_output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .map(str::to_owned)
        .ok_or_else(|| "sqlite query returned no rows".to_owned())
}

fn sender_salt_hex(value_json: &str) -> Result<String, String> {
    if value_json.len() != 66 || !value_json.starts_with('"') || !value_json.ends_with('"') {
        return Err("sender salt was not stored as a 32-byte JSON string".to_owned());
    }
    Ok(value_json[1..value_json.len() - 1].to_owned())
}

fn assert_sender_salt_debug_redacted(value: &str, salt_json: &str) -> Result<(), String> {
    let salt_hex = sender_salt_hex(salt_json)?;
    let salt_bytes = salt_hex_bytes(&salt_hex)?;
    assert!(
        value.contains("redacted sender identity salt"),
        "adapter Debug did not include the sender salt redaction marker: {value}"
    );
    assert!(
        !value.contains(&salt_hex),
        "adapter Debug leaked sender salt hex {salt_hex}: {value}"
    );
    assert!(
        !value.contains(&format!("{salt_bytes:?}")),
        "adapter Debug leaked sender salt bytes: {value}"
    );
    Ok(())
}

fn salt_hex_bytes(value: &str) -> Result<Vec<u8>, String> {
    (0..value.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&value[index..index + 2], 16).map_err(|error| error.to_string())
        })
        .collect()
}

fn legacy_messages_path_salt_hex(db_path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"morrow.list-intake.sender-salt.v1:");
    hasher.update(db_path.to_string_lossy().as_bytes());
    format!("{:x}", hasher.finalize())
}

fn labels_by_message(report: &IngestionReport) -> BTreeMap<String, String> {
    report
        .sender_groups
        .iter()
        .map(|group| {
            (
                group.message_guid.as_str().to_owned(),
                group.display_label.as_str().to_owned(),
            )
        })
        .collect()
}

fn assert_private_sender_tokens_absent(value: &str) {
    for forbidden in [
        "+15555550103",
        "ALPHA@example.COM",
        "alpha@example.com",
        "2B3135353535353530313033",
        "414C504841406578616D706C652E434F4D",
        "messages-participant-",
        "senderKey-",
    ] {
        assert!(
            !value.contains(forbidden),
            "sender report leaked forbidden token {forbidden}: {value}"
        );
    }
}

const fn apple_nanoseconds(unix_seconds: i64) -> i64 {
    (unix_seconds - 978_307_200) * 1_000_000_000
}
