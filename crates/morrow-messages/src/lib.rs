#![forbid(unsafe_code)]

mod discovery;
mod error;
mod ingest;
mod request;
mod types;
mod validation;

pub use discovery::{
    DiscoveredChat, DiscoveredChatParts, MessagesDiscoveryDataSource, MessagesDiscoveryReport,
    MessagesDiscoveryStatus,
};
pub use error::MessagesError;
pub use ingest::ingest_selected_threads;
pub use request::{BackfillDays, IngestionRequest, NativeReadRequest, WhitelistedChat};
pub use types::{
    ChatGuid, IngestionReport, IngestionStatus, MessageEvidence, MessageGuid, MessageSenderGroup,
    MessageSenderIdentity, MessageTimestamp, MessagesDataSource, NativeBatch, ParticipantId,
    PausedChat, RawChat, RawMessage, SenderDisplayLabel, SenderKey, TapbackKind,
};
