use std::{
    path::Path,
    process::{Command, Stdio},
};

pub fn create_fixture(db_path: &Path) -> Result<(), String> {
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
            VALUES (1, 'iMessage;-;chat-alpha', 'Clinic Ops'),
                   (2, 'iMessage;-;+15555550103', 'patient@example.com');
        INSERT INTO handle (ROWID, id)
            VALUES (1, '+15555550101'), (2, 'alpha@example.com'), (3, '+15555550103');
        INSERT INTO chat_handle_join (chat_id, handle_id) VALUES (1, 1), (1, 2), (2, 3);
        INSERT INTO message (ROWID, guid, date, text, attributedBody, handle_id)
            VALUES (1, 'alpha-old-message', {alpha_old}, 'alpha old body', NULL, 1),
                   (2, 'alpha-message', {alpha_latest}, 'alpha private body', NULL, 2),
                   (3, 'raw-guid-message', {beta_latest}, 'beta selected body', NULL, 3);
        INSERT INTO chat_message_join (chat_id, message_id) VALUES (1, 1), (1, 2), (2, 3);
        ",
        alpha_old = apple_nanoseconds(1_700_000_000),
        alpha_latest = apple_nanoseconds(1_700_000_100),
        beta_latest = apple_nanoseconds(1_700_000_200),
    );
    run_sqlite(db_path, &sql)
}

pub fn run_sqlite(db_path: &Path, sql: &str) -> Result<(), String> {
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
