use std::collections::BTreeMap;

use morrow_messages::{
    ChatGuid, MessageGuid, MessageSenderIdentity, MessageTimestamp, MessagesError, NativeBatch,
    NativeReadRequest, RawChat, RawMessage,
};

use super::sender_identity::{sender_identity, SenderIdentityCacheKey, SenderIdentityRows};
use super::timestamp::apple_timestamp_to_unix_seconds;
use super::{
    hex_text, message_text, parse_participant_ids, row_i64, row_u16, row_value, SenderIdentitySalt,
};

pub(super) struct ReadRecentRows {
    pub(super) batch: NativeBatch,
    pub(super) sender_identities: SenderIdentityRows,
}

pub(super) fn rows_to_batch(
    rows: &[Vec<String>],
    request: &NativeReadRequest,
    sender_salt: &SenderIdentitySalt,
) -> Result<ReadRecentRows, MessagesError> {
    let mut chats = BTreeMap::<String, RawChat>::new();
    let mut sender_identities = SenderIdentityRows::default();
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
        let message_guid = MessageGuid::parse(row_value(row, 3, "message_guid")?)?;
        let message = RawMessage {
            chat_guid: chat_guid.clone(),
            message_guid: message_guid.clone(),
            timestamp,
            text: message_text(
                row_value(row, 5, "text_hex")?,
                row_value(row, 6, "attributed_body_hex")?,
            )?,
            tapback: None,
        };
        sender_identities.insert(
            SenderIdentityCacheKey::new(&chat_guid, &message_guid),
            parse_sender_identity(
                sender_salt,
                &chat_guid,
                row_value(row, 7, "sender_handle_id")?,
                row_value(row, 8, "sender_handle_hex")?,
            )?,
        );
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
    Ok(ReadRecentRows {
        batch: NativeBatch {
            chats: chats.into_values().collect(),
        },
        sender_identities,
    })
}

fn parse_sender_identity(
    sender_salt: &SenderIdentitySalt,
    chat_guid: &ChatGuid,
    sender_handle_id: &str,
    sender_handle_hex: &str,
) -> Result<MessageSenderIdentity, MessagesError> {
    if sender_handle_id.is_empty() || sender_handle_hex.is_empty() {
        return Ok(MessageSenderIdentity::unknown());
    }
    let raw_handle = hex_text(sender_handle_hex)?;
    Ok(sender_identity(sender_salt, chat_guid, &raw_handle))
}
