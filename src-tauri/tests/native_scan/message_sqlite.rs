use std::{path::Path, process::Command};

pub(super) fn create_messages_fixture(db_path: &Path) -> Result<(), String> {
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
            VALUES (1, 'beta-provider-route', {}, 'Maybe meet tomorrow?', NULL, 3);
        INSERT INTO chat_message_join (chat_id, message_id) VALUES (1, 1);
        ",
        apple_nanoseconds(1_782_352_400)
    );
    run_sqlite(db_path, &sql)
}

pub(super) fn external_mapping_count(db_path: &Path) -> Result<i64, String> {
    query_sqlite_i64(
        db_path,
        "SELECT COUNT(*) FROM external_object_mappings WHERE source = 'calendar';",
    )
}

pub(super) fn candidate_external_receipt_count(db_path: &Path) -> Result<i64, String> {
    query_sqlite_i64(
        db_path,
        "SELECT COUNT(*) FROM candidates WHERE external_object_id IS NOT NULL AND external_source_id IS NOT NULL;",
    )
}

pub(super) fn candidate_reasons(db_path: &Path) -> Result<String, String> {
    query_sqlite(
        db_path,
        "SELECT current_reason FROM candidates UNION ALL SELECT reason FROM audit_log ORDER BY 1;",
    )
}

pub(super) fn install_external_mapping_failure_trigger(db_path: &Path) -> Result<(), String> {
    run_sqlite(
        db_path,
        "CREATE TRIGGER fail_external_mapping_insert BEFORE INSERT ON external_object_mappings BEGIN SELECT RAISE(FAIL, 'simulated post-create storage failure'); END;",
    )
}

pub(super) fn drop_external_mapping_failure_trigger(db_path: &Path) -> Result<(), String> {
    run_sqlite(db_path, "DROP TRIGGER fail_external_mapping_insert;")
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

fn query_sqlite(db_path: &Path, sql: &str) -> Result<String, String> {
    let output = Command::new("sqlite3")
        .arg("-batch")
        .arg("-noheader")
        .arg(db_path)
        .arg(sql)
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

fn query_sqlite_i64(db_path: &Path, sql: &str) -> Result<i64, String> {
    query_sqlite(db_path, sql)?
        .trim()
        .parse::<i64>()
        .map_err(|error| error.to_string())
}

const fn apple_nanoseconds(unix_seconds: i64) -> i64 {
    (unix_seconds - 978_307_200) * 1_000_000_000
}
