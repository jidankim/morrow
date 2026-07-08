use std::{path::Path, process::Command};

use morrow_lib::native_bridge::messages_sqlite::{MessagesSqliteAdapter, MessagesSqliteLimits};
use morrow_messages::{ChatGuid, MessageTimestamp, MessagesDataSource, NativeReadRequest};

const ATTRIBUTED_BODY_HEX: &str = concat!(
    "040b73747265616d747970656481e803840140848484124e5341747472696275746564537472",
    "696e67008484084e534f626a656374008592848484084e53537472696e67019484012b3a4d",
    "6f72726f7720514120617474726962757465642066616c6c6261636b206d656574696e6720",
    "746f6d6f72726f7720617420333a333020504d2e8684026949013a928484840c4e534469",
    "6374696f6e6172790094840169008686",
);

#[test]
fn read_recent_sql_filters_to_native_window_before_row_number() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join("chat.db");
    create_read_window_fixture(&db_path)?;
    let adapter = MessagesSqliteAdapter::new(db_path).with_limits(MessagesSqliteLimits {
        discovery_chat_limit: 10,
        read_message_limit: 4,
    });
    let request = NativeReadRequest {
        chat_guids: vec![
            ChatGuid::parse("iMessage;-;read-window").map_err(|error| error.to_string())?
        ],
        since: MessageTimestamp::new(1_700_000_100).map_err(|error| error.to_string())?,
        until: MessageTimestamp::new(1_700_000_300).map_err(|error| error.to_string())?,
    };

    // When
    let batch = adapter
        .read_recent(&request)
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(batch.chats.len(), 1);
    let chat = &batch.chats[0];
    assert_eq!(chat.guid.as_str(), "iMessage;-;read-window");
    assert_eq!(chat.participant_count, 2);
    assert_eq!(
        chat.participant_ids
            .iter()
            .map(|participant| participant.as_str())
            .collect::<Vec<_>>(),
        vec![
            "messages-participant-02e3595aeac728ed101243e18982199f",
            "messages-participant-356ffd49fdd57f2c4d88762c2a2477ae",
        ]
    );
    assert_eq!(
        chat.messages
            .iter()
            .map(|message| message.message_guid.as_str())
            .collect::<Vec<_>>(),
        vec![
            "since-boundary",
            "inside-window-older",
            "inside-window-newer",
            "until-boundary",
        ]
    );
    assert_eq!(
        chat.messages
            .iter()
            .map(|message| message.text.as_str())
            .collect::<Vec<_>>(),
        vec![
            "inside since boundary",
            "Morrow QA attributed fallback meeting tomorrow at 3:30 PM.",
            "inside newer",
            "inside until boundary",
        ]
    );
    assert!(chat
        .messages
        .iter()
        .all(|message| message.timestamp.as_i64() >= 1_700_000_100
            && message.timestamp.as_i64() <= 1_700_000_300));
    assert!(chat
        .messages
        .iter()
        .all(|message| message.message_guid.as_str() != "future-outside-window"));
    assert!(chat
        .messages
        .iter()
        .all(|message| message.message_guid.as_str() != "old-high-rowid-outside-window"));
    Ok(())
}

fn create_read_window_fixture(db_path: &Path) -> Result<(), String> {
    run_sqlite(
        db_path,
        &format!(
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
                VALUES (1, 'iMessage;-;read-window', 'Read Window');
            INSERT INTO handle (ROWID, id) VALUES (1, '+15555550101'), (2, '+15555550102');
            INSERT INTO chat_handle_join (chat_id, handle_id) VALUES (1, 1), (1, 2);

            INSERT INTO message (ROWID, guid, date, text, attributedBody, handle_id) VALUES
                (10, 'future-outside-window', {}, 'recent-looking outside window', NULL, 1),
                (15, 'until-boundary', {}, 'inside until boundary', NULL, 1),
                (20, 'inside-window-newer', {}, 'inside newer', NULL, 1),
                (30, 'inside-window-older', {}, NULL, X'{}', 2),
                (40, 'since-boundary', {}, 'inside since boundary', NULL, 2),
                (999, 'old-high-rowid-outside-window', {}, 'old outside window', NULL, 2);
            INSERT INTO chat_message_join (chat_id, message_id) VALUES
                (1, 10),
                (1, 15),
                (1, 20),
                (1, 30),
                (1, 40),
                (1, 999);
            ",
            apple_nanoseconds(1_700_000_400),
            apple_nanoseconds(1_700_000_300),
            apple_nanoseconds(1_700_000_250),
            apple_nanoseconds(1_700_000_150),
            ATTRIBUTED_BODY_HEX,
            apple_nanoseconds(1_700_000_100),
            apple_nanoseconds(1_699_999_900),
        ),
    )
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
