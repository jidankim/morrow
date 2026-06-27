use morrow_messages::{
    BackfillDays, ChatGuid, IngestionRequest, MessageTimestamp, MessagesDataSource, MessagesError,
    NativeBatch, NativeReadRequest, WhitelistedChat,
};

use super::{messages_error, ScanSelectedChatsError, ScanSelectedChatsRequest};

pub(super) struct UnavailableMessagesDataSource;

impl MessagesDataSource for UnavailableMessagesDataSource {
    fn read_recent(&self, _request: &NativeReadRequest) -> Result<NativeBatch, MessagesError> {
        Err(MessagesError::PermissionDenied)
    }
}

pub(super) fn ingestion_request(
    request: &ScanSelectedChatsRequest,
) -> Result<IngestionRequest, ScanSelectedChatsError> {
    let whitelist = request
        .selected_chat_ids
        .iter()
        .map(|chat_id| {
            WhitelistedChat::new(ChatGuid::parse(chat_id).map_err(messages_error)?, 3)
                .map_err(messages_error)
        })
        .collect::<Result<Vec<_>, _>>()?;
    IngestionRequest::new(
        whitelist,
        BackfillDays::new(7).map_err(messages_error)?,
        MessageTimestamp::new(1_782_352_400).map_err(messages_error)?,
    )
    .map_err(messages_error)
}
