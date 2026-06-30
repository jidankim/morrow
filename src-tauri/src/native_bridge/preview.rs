use serde::{Deserialize, Serialize};

const MESSAGE_PREVIEW_VISIBLE_CHAR_LIMIT: usize = 120;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MessagesPreviewRequest {
    pub chat_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MessagesPreviewCommandReport {
    pub chats: Vec<MessagesPreviewCommandChat>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MessagesPreviewCommandChat {
    pub chat_id: String,
    pub preview: String,
}

impl MessagesPreviewCommandReport {
    pub fn empty() -> Self {
        Self { chats: Vec::new() }
    }
}

impl MessagesPreviewCommandChat {
    pub fn new(chat_id: String, preview: String) -> Self {
        Self { chat_id, preview }
    }
}

pub(crate) fn normalize_message_preview(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= MESSAGE_PREVIEW_VISIBLE_CHAR_LIMIT {
        return collapsed;
    }
    let mut preview = collapsed
        .chars()
        .take(MESSAGE_PREVIEW_VISIBLE_CHAR_LIMIT)
        .collect::<String>();
    preview.truncate(preview.trim_end().len());
    preview.push_str("...");
    preview
}
