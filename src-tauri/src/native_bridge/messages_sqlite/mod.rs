mod queries;
mod sqlite_cli;
mod timestamp;

use std::{collections::BTreeMap, path::PathBuf};

use morrow_messages::{
    ChatGuid, DiscoveredChat, DiscoveredChatParts, MessageGuid, MessageTimestamp,
    MessagesDataSource, MessagesDiscoveryDataSource, MessagesDiscoveryReport, MessagesError,
    NativeBatch, NativeReadRequest, ParticipantId, RawChat, RawMessage,
};

use super::public_chat_id::{public_chat_id, public_participant_id};
use queries::{all_chat_guids_sql, discovery_sql, read_recent_sql};
pub use sqlite_cli::SqliteReadProtections;
use sqlite_cli::{sqlite_error_from_stderr, Sqlite};
use timestamp::apple_timestamp_to_unix_seconds;

const DEFAULT_DISCOVERY_CHAT_LIMIT: u16 = 50;
const DEFAULT_READ_MESSAGE_LIMIT: u16 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessagesSqliteLimits {
    pub discovery_chat_limit: u16,
    pub read_message_limit: u16,
}

impl Default for MessagesSqliteLimits {
    fn default() -> Self {
        Self {
            discovery_chat_limit: DEFAULT_DISCOVERY_CHAT_LIMIT,
            read_message_limit: DEFAULT_READ_MESSAGE_LIMIT,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MessagesSqliteAdapter {
    db_path: PathBuf,
    limits: MessagesSqliteLimits,
    runner: MessagesSqliteRunner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MessagesSqliteRunner {
    Cli,
    PermissionDeniedStderr,
}

impl MessagesSqliteAdapter {
    pub fn new(db_path: PathBuf) -> Self {
        Self {
            db_path,
            limits: MessagesSqliteLimits::default(),
            runner: MessagesSqliteRunner::Cli,
        }
    }

    #[doc(hidden)]
    pub fn permission_denied_for_test(db_path: PathBuf) -> Self {
        Self {
            db_path,
            limits: MessagesSqliteLimits::default(),
            runner: MessagesSqliteRunner::PermissionDeniedStderr,
        }
    }

    pub const fn with_limits(mut self, limits: MessagesSqliteLimits) -> Self {
        self.limits = limits;
        self
    }

    pub const fn sqlite_read_protections() -> SqliteReadProtections {
        Sqlite::protections()
    }

    fn sqlite(&self) -> Sqlite {
        Sqlite::new(self.db_path.clone())
    }

    fn query_rows(&self, sql: &str) -> Result<Vec<Vec<String>>, MessagesError> {
        match self.runner {
            MessagesSqliteRunner::Cli => self.sqlite().query_rows(sql),
            MessagesSqliteRunner::PermissionDeniedStderr => {
                Err(sqlite_error_from_stderr(b"Error: permission denied"))
            }
        }
    }

    pub fn resolve_public_chat_ids(
        &self,
        public_ids: &[String],
    ) -> Result<Vec<ChatGuid>, MessagesError> {
        let rows = self.query_rows(all_chat_guids_sql())?;
        let mut guid_by_public_id = BTreeMap::new();
        for row in rows {
            let chat_guid = ChatGuid::parse(row_value(&row, 0, "chat_guid")?)?;
            guid_by_public_id.insert(public_chat_id(&chat_guid), chat_guid);
        }
        public_ids
            .iter()
            .map(|public_id| {
                guid_by_public_id.get(public_id).cloned().ok_or_else(|| {
                    MessagesError::InvalidInput {
                        field: "selected_chat_ids",
                        reason: "contains an unknown Messages chat id".to_owned(),
                    }
                })
            })
            .collect()
    }
}

impl MessagesDiscoveryDataSource for MessagesSqliteAdapter {
    fn discover_chats(&self) -> Result<MessagesDiscoveryReport, MessagesError> {
        let rows = match self.query_rows(&discovery_sql(self.limits.discovery_chat_limit)) {
            Ok(rows) => rows,
            Err(MessagesError::PermissionDenied) => {
                return Ok(MessagesDiscoveryReport::permission_denied())
            }
            Err(MessagesError::NativeUnavailable { .. }) => {
                return Ok(MessagesDiscoveryReport::unavailable())
            }
            Err(err @ MessagesError::InvalidInput { .. }) => return Err(err),
        };
        if rows.is_empty() {
            return Ok(MessagesDiscoveryReport::empty());
        }
        let chats = rows
            .iter()
            .map(|row| parse_discovered_chat(row))
            .collect::<Result<Vec<_>, _>>()?;
        MessagesDiscoveryReport::ready(chats)
    }
}

impl MessagesDataSource for MessagesSqliteAdapter {
    fn read_recent(&self, request: &NativeReadRequest) -> Result<NativeBatch, MessagesError> {
        if request.chat_guids.is_empty() {
            return Ok(NativeBatch::default());
        }
        let sql = read_recent_sql(&request.chat_guids, self.limits.read_message_limit)?;
        let rows = self.query_rows(&sql)?;
        rows_to_batch(&rows, request)
    }
}

fn parse_discovered_chat(row: &[String]) -> Result<DiscoveredChat, MessagesError> {
    let latest_timestamp = apple_timestamp_to_unix_seconds(row_i64(row, 2, "latest_date")?)?;
    let participant_count = row_u16(row, 3, "participant_count")?;
    let participant_ids = parse_public_participant_ids(row_value(row, 4, "participant_handles")?)?;
    let participant_id_refs = participant_ids
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    DiscoveredChat::parse(DiscoveredChatParts {
        chat_guid: row_value(row, 0, "chat_guid")?,
        display_label: &hex_text(row_value(row, 1, "display_label_hex")?)?,
        participant_count,
        participant_ids: participant_id_refs,
        latest_activity_timestamp: latest_timestamp,
    })
}

fn rows_to_batch(
    rows: &[Vec<String>],
    request: &NativeReadRequest,
) -> Result<NativeBatch, MessagesError> {
    let mut chats = BTreeMap::<String, RawChat>::new();
    for row in rows {
        let chat_guid = ChatGuid::parse(row_value(row, 0, "chat_guid")?)?;
        let timestamp = MessageTimestamp::new(apple_timestamp_to_unix_seconds(row_i64(
            row,
            4,
            "message_date",
        )?)?)?;
        if timestamp < request.since || timestamp > request.until {
            continue;
        }
        let participant_count = row_u16(row, 1, "participant_count")?;
        let participant_ids = parse_participant_ids(row_value(row, 2, "participant_handles")?)?;
        let message = RawMessage {
            chat_guid: chat_guid.clone(),
            message_guid: MessageGuid::parse(row_value(row, 3, "message_guid")?)?,
            timestamp,
            text: hex_text(row_value(row, 5, "text_hex")?)?,
            tapback: None,
        };
        chats
            .entry(chat_guid.as_str().to_owned())
            .or_insert_with(|| RawChat {
                guid: chat_guid,
                participant_count,
                participant_ids,
                messages: Vec::new(),
            })
            .messages
            .push(message);
    }
    Ok(NativeBatch {
        chats: chats.into_values().collect(),
    })
}

fn parse_participant_ids(raw: &str) -> Result<Vec<ParticipantId>, MessagesError> {
    parse_public_participant_ids(raw)?
        .iter()
        .map(|value| ParticipantId::parse(value))
        .collect()
}

fn parse_public_participant_ids(raw: &str) -> Result<Vec<String>, MessagesError> {
    raw.split(',')
        .filter(|value| !value.is_empty())
        .map(|value| hex_text(value).map(|raw_handle| public_participant_id(&raw_handle)))
        .collect()
}

fn row_value<'a>(
    row: &'a [String],
    index: usize,
    field: &'static str,
) -> Result<&'a str, MessagesError> {
    row.get(index)
        .map(String::as_str)
        .ok_or_else(|| unavailable(format!("sqlite row missing {field}")))
}

fn row_i64(row: &[String], index: usize, field: &'static str) -> Result<i64, MessagesError> {
    row_value(row, index, field)?
        .parse::<i64>()
        .map_err(|error| unavailable(format!("sqlite row has invalid {field}: {error}")))
}

fn row_u16(row: &[String], index: usize, field: &'static str) -> Result<u16, MessagesError> {
    let value = row_i64(row, index, field)?;
    u16::try_from(value)
        .map_err(|error| unavailable(format!("sqlite row has invalid {field}: {error}")))
}

fn hex_text(value: &str) -> Result<String, MessagesError> {
    let bytes = value.as_bytes();
    if bytes.len() % 2 != 0 {
        return Err(unavailable("sqlite returned odd-length hex text"));
    }
    let mut decoded = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let high =
            hex_nibble(pair[0]).ok_or_else(|| unavailable("sqlite returned invalid hex text"))?;
        let low =
            hex_nibble(pair[1]).ok_or_else(|| unavailable("sqlite returned invalid hex text"))?;
        decoded.push((high << 4) | low);
    }
    String::from_utf8(decoded)
        .map_err(|error| unavailable(format!("sqlite returned non-utf8 text: {error}")))
}

const fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn unavailable(reason: impl Into<String>) -> MessagesError {
    MessagesError::NativeUnavailable {
        reason: reason.into(),
    }
}
