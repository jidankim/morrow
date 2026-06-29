use std::collections::BTreeSet;

use morrow_messages::{
    BackfillDays, ChatGuid, IngestionRequest, MessageTimestamp, MessagesError, ParticipantId,
    WhitelistedChat,
};

use super::{
    messages_error, ScanSelectedChatsError, ScanSelectedChatsRequest, SelectedChatMetadata,
};

pub(super) fn ingestion_request(
    request: &ScanSelectedChatsRequest,
    reference_unix_seconds: i64,
) -> Result<IngestionRequest, ScanSelectedChatsError> {
    validate_selected_chat_metadata(request)?;
    let whitelist = request
        .selected_chats
        .iter()
        .map(whitelisted_chat)
        .collect::<Result<Vec<_>, _>>()?;
    IngestionRequest::new(
        whitelist,
        BackfillDays::new(7).map_err(messages_error)?,
        MessageTimestamp::new(reference_unix_seconds).map_err(messages_error)?,
    )
    .map_err(messages_error)
}

fn validate_selected_chat_metadata(
    request: &ScanSelectedChatsRequest,
) -> Result<(), ScanSelectedChatsError> {
    if request.selected_chats.is_empty() {
        return Err(invalid_input(
            "selected_chats",
            "must include at least one chat",
        ));
    }
    if request.selected_chat_ids.len() != request.selected_chats.len() {
        return Err(invalid_input(
            "selected_chat_ids",
            "must match selected_chats",
        ));
    }
    let mut seen = BTreeSet::new();
    for (selected_id, chat) in request
        .selected_chat_ids
        .iter()
        .zip(&request.selected_chats)
    {
        if selected_id != &chat.id {
            return Err(invalid_input(
                "selected_chats",
                "must align with selected_chat_ids",
            ));
        }
        if !seen.insert(chat.id.as_str()) {
            return Err(invalid_input(
                "selected_chats",
                "must not contain duplicate ids",
            ));
        }
    }
    Ok(())
}

fn whitelisted_chat(
    chat: &SelectedChatMetadata,
) -> Result<WhitelistedChat, ScanSelectedChatsError> {
    let participant_ids = chat
        .participant_ids
        .iter()
        .map(|id| ParticipantId::parse(id).map_err(messages_error))
        .collect::<Result<Vec<_>, _>>()?;
    WhitelistedChat::with_participants(
        ChatGuid::parse(&chat.id).map_err(messages_error)?,
        chat.participant_count,
        participant_ids,
    )
    .map_err(messages_error)
}

fn invalid_input(field: &'static str, reason: &'static str) -> ScanSelectedChatsError {
    messages_error(MessagesError::InvalidInput {
        field,
        reason: reason.to_owned(),
    })
}
