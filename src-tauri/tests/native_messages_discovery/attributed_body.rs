use std::{
    path::Path,
    process::{Command, Stdio},
};

use morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter;
use morrow_messages::{ChatGuid, MessageTimestamp, MessagesDataSource, NativeReadRequest};

#[test]
fn reads_recent_messages_from_attributed_body_when_text_column_is_empty() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join("chat.db");
    create_attributed_body_fixture(&db_path)?;
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
    assert_eq!(batch.chats[0].messages.len(), 1);
    assert_eq!(
        batch.chats[0].messages[0].text,
        "Morrow QA attributed fallback meeting tomorrow at 3:30 PM."
    );
    Ok(())
}

fn create_attributed_body_fixture(db_path: &Path) -> Result<(), String> {
    const ATTRIBUTED_BODY_HEX: &str = concat!(
        "040b73747265616d747970656481e803840140848484124e5341747472696275746564537472",
        "696e67008484084e534f626a656374008592848484084e53537472696e67019484012b3a4d",
        "6f72726f7720514120617474726962757465642066616c6c6261636b206d656574696e6720",
        "746f6d6f72726f7720617420333a333020504d2e8684026949013a928484840c4e534469",
        "6374696f6e6172790094840169008686",
    );
    let sql = format!(
        concat!(
            "CREATE TABLE chat (ROWID INTEGER PRIMARY KEY, guid TEXT NOT NULL, display_name TEXT);",
            "CREATE TABLE handle (ROWID INTEGER PRIMARY KEY, id TEXT NOT NULL);",
            "CREATE TABLE message (ROWID INTEGER PRIMARY KEY, guid TEXT NOT NULL, date INTEGER NOT NULL, ",
            "text TEXT, attributedBody BLOB, handle_id INTEGER);",
            "CREATE TABLE chat_message_join (chat_id INTEGER NOT NULL, message_id INTEGER NOT NULL);",
            "CREATE TABLE chat_handle_join (chat_id INTEGER NOT NULL, handle_id INTEGER NOT NULL);",
            "INSERT INTO chat (ROWID, guid, display_name) VALUES ",
            "(1, 'iMessage;-;+15555550103', 'Messages chat');",
            "INSERT INTO handle (ROWID, id) VALUES (3, '+15555550103');",
            "INSERT INTO chat_handle_join (chat_id, handle_id) VALUES (1, 3);",
            "INSERT INTO message (ROWID, guid, date, text, attributedBody, handle_id) VALUES ",
            "(1, 'attributed-message', {latest}, NULL, X'{attributed_body_hex}', 3);",
            "INSERT INTO chat_message_join (chat_id, message_id) VALUES (1, 1);",
        ),
        latest = apple_nanoseconds(1_700_000_200),
        attributed_body_hex = ATTRIBUTED_BODY_HEX,
    );
    run_sqlite(db_path, &sql)
}

fn run_sqlite(db_path: &Path, sql: &str) -> Result<(), String> {
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
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

const fn apple_nanoseconds(unix_seconds: i64) -> i64 {
    (unix_seconds - 978_307_200) * 1_000_000_000
}
