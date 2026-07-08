use morrow_messages::{ChatGuid, MessagesError};

use super::{message_text, queries::latest_previews_sql, row_value, MessagesSqliteAdapter};
use crate::native_bridge::{
    preview::{
        normalize_message_preview, MessagesPreviewCommandChat, MessagesPreviewCommandReport,
        MessagesPreviewRequest,
    },
    public_chat_id::public_chat_id,
};

impl MessagesSqliteAdapter {
    pub fn load_messages_chat_previews(
        &self,
        request: &MessagesPreviewRequest,
    ) -> Result<MessagesPreviewCommandReport, MessagesError> {
        if request.chat_ids.is_empty() {
            return Ok(MessagesPreviewCommandReport::empty());
        }
        let chat_guids = self.resolve_public_chat_ids(&request.chat_ids)?;
        self.load_messages_chat_previews_for_chat_guids(&chat_guids)
    }

    pub(in crate::native_bridge) fn load_messages_chat_previews_for_chat_guids(
        &self,
        chat_guids: &[ChatGuid],
    ) -> Result<MessagesPreviewCommandReport, MessagesError> {
        if chat_guids.is_empty() {
            return Ok(MessagesPreviewCommandReport::empty());
        }
        let rows = self.query_rows(&latest_previews_sql(&chat_guids)?)?;
        preview_rows_to_report(&rows)
    }
}

fn preview_rows_to_report(
    rows: &[Vec<String>],
) -> Result<MessagesPreviewCommandReport, MessagesError> {
    let chats = rows
        .iter()
        .map(|row| {
            let chat_guid = ChatGuid::parse(row_value(row, 0, "chat_guid")?)?;
            let text = message_text(
                row_value(row, 1, "text_hex")?,
                row_value(row, 2, "attributed_body_hex")?,
            )?;
            Ok(MessagesPreviewCommandChat::new(
                public_chat_id(&chat_guid),
                normalize_message_preview(&text),
            ))
        })
        .collect::<Result<Vec<_>, MessagesError>>()?;
    Ok(MessagesPreviewCommandReport { chats })
}
