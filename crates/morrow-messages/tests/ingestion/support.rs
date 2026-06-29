use morrow_messages::{
    BackfillDays, ChatGuid, IngestionRequest, MessageGuid, MessageTimestamp, MessagesDataSource,
    MessagesError, NativeBatch, NativeReadRequest, RawMessage, TapbackKind, WhitelistedChat,
};

#[derive(Debug, Default)]
pub(super) struct FakeMessages {
    batch: NativeBatch,
    error: Option<MessagesError>,
    calls: std::cell::Cell<usize>,
}

impl FakeMessages {
    pub(super) fn with_batch(batch: NativeBatch) -> Self {
        Self {
            batch,
            error: None,
            calls: std::cell::Cell::new(0),
        }
    }

    pub(super) fn permission_denied() -> Self {
        Self {
            batch: NativeBatch::default(),
            error: Some(MessagesError::PermissionDenied),
            calls: std::cell::Cell::new(0),
        }
    }

    pub(super) fn calls(&self) -> usize {
        self.calls.get()
    }
}

impl MessagesDataSource for FakeMessages {
    fn read_recent(&self, _request: &NativeReadRequest) -> Result<NativeBatch, MessagesError> {
        self.calls.set(self.calls.get() + 1);
        match &self.error {
            Some(err) => Err(err.clone()),
            None => Ok(self.batch.clone()),
        }
    }
}

pub(super) fn request_with_whitelist(
    chat_guid: &str,
    participant_count: u16,
    backfill_days: u8,
    now: i64,
) -> IngestionRequest {
    IngestionRequest::new(
        vec![WhitelistedChat::new(
            ChatGuid::parse(chat_guid).expect("chat guid"),
            participant_count,
        )
        .expect("whitelisted chat")],
        BackfillDays::new(backfill_days).expect("backfill"),
        MessageTimestamp::new(now).expect("now"),
    )
    .expect("request")
}

pub(super) fn message(
    chat_guid: &str,
    message_guid: &str,
    timestamp: i64,
    text: &str,
    tapback: Option<TapbackKind>,
) -> RawMessage {
    RawMessage {
        chat_guid: ChatGuid::parse(chat_guid).expect("chat guid"),
        message_guid: MessageGuid::parse(message_guid).expect("message guid"),
        timestamp: MessageTimestamp::new(timestamp).expect("timestamp"),
        text: text.to_owned(),
        tapback,
    }
}
