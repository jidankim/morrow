use std::cell::Cell;
use std::error::Error;

use morrow_messages::{
    ingest_selected_threads, BackfillDays, ChatGuid, IngestionRequest, IngestionStatus,
    MessageGuid, MessageTimestamp, MessagesDataSource, MessagesError, NativeBatch,
    NativeReadRequest, ParticipantId, RawChat, RawMessage, WhitelistedChat,
};

#[derive(Debug, Clone)]
pub(crate) struct MessagesSummary {
    pub(crate) permission_status: &'static str,
    pub(crate) permission_warning: String,
    pub(crate) unavailable_warning: String,
    pub(crate) unavailable_messages: usize,
    pub(crate) guid_change_warning: String,
    pub(crate) guid_change_messages: usize,
    pub(crate) group_pause_reason: String,
    pub(crate) group_messages: usize,
}

#[derive(Debug)]
struct FakeMessages {
    batch: NativeBatch,
    error: Option<MessagesError>,
    calls: Cell<usize>,
}

impl FakeMessages {
    const fn with_batch(batch: NativeBatch) -> Self {
        Self {
            batch,
            error: None,
            calls: Cell::new(0),
        }
    }

    fn permission_denied() -> Self {
        Self {
            batch: NativeBatch::default(),
            error: Some(MessagesError::PermissionDenied),
            calls: Cell::new(0),
        }
    }
}

impl MessagesDataSource for FakeMessages {
    fn read_recent(&self, _request: &NativeReadRequest) -> Result<NativeBatch, MessagesError> {
        self.calls.set(self.calls.get() + 1);
        self.error
            .as_ref()
            .map_or_else(|| Ok(self.batch.clone()), |error| Err(error.clone()))
    }
}

pub(crate) fn run() -> Result<MessagesSummary, Box<dyn Error>> {
    let permission = ingest_selected_threads(
        &FakeMessages::permission_denied(),
        &request("chat-alpha", 2)?,
    )?;
    let unavailable = ingest_selected_threads(
        &FakeMessages::with_batch(NativeBatch::default()),
        &request("chat-alpha", 2)?,
    )?;
    let guid_changed = ingest_selected_threads(
        &FakeMessages::with_batch(NativeBatch {
            chats: vec![chat("chat-renamed", 2, Vec::new(), "msg-renamed")?],
        }),
        &request("chat-alpha", 2)?,
    )?;
    let group = ingest_selected_threads(
        &FakeMessages::with_batch(NativeBatch {
            chats: vec![chat(
                "chat-group",
                2,
                vec![participant("participant-a")?, participant("participant-c")?],
                "msg-group",
            )?],
        }),
        &group_request()?,
    )?;
    let permission_warning = only_warning(&permission.warnings)?;
    let unavailable_warning = only_warning(&unavailable.warnings)?;
    let guid_change_warning = only_warning(&guid_changed.warnings)?;
    let group_pause_reason = group
        .paused_chats
        .first()
        .map(|paused| paused.reason.clone())
        .ok_or_else(|| std::io::Error::other("missing group pause"))?;

    Ok(MessagesSummary {
        permission_status: status_name(permission.status),
        permission_warning,
        unavailable_warning,
        unavailable_messages: unavailable.messages.len(),
        guid_change_warning,
        guid_change_messages: guid_changed.messages.len(),
        group_pause_reason,
        group_messages: group.messages.len(),
    })
}

fn request(chat_guid: &str, participant_count: u16) -> Result<IngestionRequest, MessagesError> {
    IngestionRequest::new(
        vec![WhitelistedChat::new(
            ChatGuid::parse(chat_guid)?,
            participant_count,
        )?],
        BackfillDays::new(7)?,
        MessageTimestamp::new(1_783_000_200)?,
    )
}

fn group_request() -> Result<IngestionRequest, MessagesError> {
    IngestionRequest::new(
        vec![WhitelistedChat::with_participants(
            ChatGuid::parse("chat-group")?,
            2,
            vec![participant("participant-a")?, participant("participant-b")?],
        )?],
        BackfillDays::new(7)?,
        MessageTimestamp::new(1_783_000_200)?,
    )
}

fn chat(
    chat_guid: &str,
    participant_count: u16,
    participant_ids: Vec<ParticipantId>,
    message_guid: &str,
) -> Result<RawChat, MessagesError> {
    Ok(RawChat {
        guid: ChatGuid::parse(chat_guid)?,
        participant_count,
        participant_ids,
        messages: vec![RawMessage {
            chat_guid: ChatGuid::parse(chat_guid)?,
            message_guid: MessageGuid::parse(message_guid)?,
            timestamp: MessageTimestamp::new(1_783_000_190)?,
            text: "Dentist Friday at 2".to_owned(),
            tapback: None,
        }],
    })
}

fn participant(value: &str) -> Result<ParticipantId, MessagesError> {
    ParticipantId::parse(value)
}

fn only_warning(warnings: &[String]) -> Result<String, Box<dyn Error>> {
    warnings
        .first()
        .cloned()
        .ok_or_else(|| std::io::Error::other("missing warning").into())
}

const fn status_name(status: IngestionStatus) -> &'static str {
    match status {
        IngestionStatus::Available => "available",
        IngestionStatus::Unavailable => "unavailable",
    }
}
