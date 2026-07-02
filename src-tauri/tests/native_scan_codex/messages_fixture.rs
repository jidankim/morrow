use std::{path::Path, process::Command};

pub const FIXTURE_MESSAGE_TEXT: &str = "Maybe meet tomorrow?";
pub const PRIVACY_CANARY: &str = "MORROW_PRIVACY_CANARY_RAW_TEXT";
pub const NATIVE_CHAT_ID: &str = "iMessage;-;+15555550103";
pub const NATIVE_MESSAGE_ID: &str = "beta-provider-route";

pub fn create_messages_fixture(db_path: &Path, message_unix_seconds: i64) -> Result<(), String> {
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
            VALUES (1, 'iMessage;-;+15555550103', 'Messages chat');
        INSERT INTO handle (ROWID, id) VALUES (3, '+15555550103');
        INSERT INTO chat_handle_join (chat_id, handle_id) VALUES (1, 3);
        INSERT INTO message (ROWID, guid, date, text, attributedBody, handle_id)
            VALUES (1, '{}', {}, '{} {}', NULL, 3);
        INSERT INTO chat_message_join (chat_id, message_id) VALUES (1, 1);
        ",
        NATIVE_MESSAGE_ID,
        apple_nanoseconds(message_unix_seconds),
        FIXTURE_MESSAGE_TEXT,
        PRIVACY_CANARY
    );
    run_sqlite(db_path, &sql)
}

fn run_sqlite(db_path: &Path, sql: &str) -> Result<(), String> {
    let output = Command::new("sqlite3")
        .arg(db_path)
        .arg(sql)
        .output()
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
