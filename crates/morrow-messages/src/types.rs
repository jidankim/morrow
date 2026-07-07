use crate::request::NativeReadRequest;
use crate::validation::validate_text;
use crate::MessagesError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChatGuid(String);

impl ChatGuid {
    pub fn parse(value: &str) -> Result<Self, MessagesError> {
        validate_text("chat_guid", value, 240)?;
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageGuid(String);

impl MessageGuid {
    pub fn parse(value: &str) -> Result<Self, MessagesError> {
        validate_text("message_guid", value, 240)?;
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParticipantId(String);

impl ParticipantId {
    pub fn parse(value: &str) -> Result<Self, MessagesError> {
        validate_text("participant_id", value, 240)?;
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MessageTimestamp(i64);

impl MessageTimestamp {
    pub fn new(value: i64) -> Result<Self, MessagesError> {
        if value >= 0 {
            Ok(Self(value))
        } else {
            Err(MessagesError::InvalidInput {
                field: "timestamp",
                reason: "must be non-negative".to_owned(),
            })
        }
    }

    pub const fn as_i64(self) -> i64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TapbackKind {
    Like,
    Love,
    Emphasis,
    Dislike,
    Laugh,
    Question,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawMessage {
    pub chat_guid: ChatGuid,
    pub message_guid: MessageGuid,
    pub timestamp: MessageTimestamp,
    pub text: String,
    pub tapback: Option<TapbackKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawChat {
    pub guid: ChatGuid,
    pub participant_count: u16,
    pub participant_ids: Vec<ParticipantId>,
    pub messages: Vec<RawMessage>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NativeBatch {
    pub chats: Vec<RawChat>,
}

pub trait MessagesDataSource {
    fn read_recent(&self, request: &NativeReadRequest) -> Result<NativeBatch, MessagesError>;

    fn sender_identity(
        &self,
        _chat_guid: &ChatGuid,
        _message_guid: &MessageGuid,
    ) -> MessageSenderIdentity {
        MessageSenderIdentity::unknown()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestionStatus {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageEvidence {
    pub chat_guid: ChatGuid,
    pub message_guid: MessageGuid,
    pub timestamp: MessageTimestamp,
    pub participant_count: u16,
    pub tapback_signal: bool,
    pub excerpt: String,
    pub evidence_pointer: String,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SenderKey(String);

impl SenderKey {
    pub fn from_private_digest(value: &str) -> Result<Self, MessagesError> {
        validate_text("sender_key", value, 96)?;
        Ok(Self(value.to_owned()))
    }

    pub fn unknown() -> Self {
        Self("unknownSender".to_owned())
    }

    pub fn is_unknown(&self) -> bool {
        self.0 == "unknownSender"
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SenderKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SenderKey(<local-private>)")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageSenderIdentity {
    key: SenderKey,
}

impl MessageSenderIdentity {
    pub const fn known(key: SenderKey) -> Self {
        Self { key }
    }

    pub fn unknown() -> Self {
        Self {
            key: SenderKey::unknown(),
        }
    }

    pub(crate) const fn key(&self) -> &SenderKey {
        &self.key
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SenderDisplayLabel(String);

impl SenderDisplayLabel {
    pub fn known(index: usize) -> Self {
        Self(format!("Sender {index}"))
    }

    pub fn unknown() -> Self {
        Self("Unknown sender".to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageSenderGroup {
    pub chat_guid: ChatGuid,
    pub message_guid: MessageGuid,
    pub display_label: SenderDisplayLabel,
    sender_key: SenderKey,
}

impl MessageSenderGroup {
    pub(crate) const fn new(
        chat_guid: ChatGuid,
        message_guid: MessageGuid,
        display_label: SenderDisplayLabel,
        sender_key: SenderKey,
    ) -> Self {
        Self {
            chat_guid,
            message_guid,
            display_label,
            sender_key,
        }
    }

    pub fn sender_key(&self) -> &SenderKey {
        &self.sender_key
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PausedChat {
    pub chat_guid: ChatGuid,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestionReport {
    pub status: IngestionStatus,
    pub messages: Vec<MessageEvidence>,
    pub sender_groups: Vec<MessageSenderGroup>,
    pub paused_chats: Vec<PausedChat>,
    pub warnings: Vec<String>,
}

impl IngestionReport {
    pub(crate) fn available() -> Self {
        Self {
            status: IngestionStatus::Available,
            messages: Vec::new(),
            sender_groups: Vec::new(),
            paused_chats: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub(crate) fn unavailable(warning: &str) -> Self {
        Self {
            status: IngestionStatus::Unavailable,
            messages: Vec::new(),
            sender_groups: Vec::new(),
            paused_chats: Vec::new(),
            warnings: vec![warning.to_owned()],
        }
    }
}
