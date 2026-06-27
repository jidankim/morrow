#![forbid(unsafe_code)]

mod error;
mod ingest;
mod request;
mod types;
mod validation;

pub use error::MessagesError;
pub use ingest::ingest_selected_threads;
pub use request::{BackfillDays, IngestionRequest, NativeReadRequest, WhitelistedChat};
pub use types::{
    ChatGuid, IngestionReport, IngestionStatus, MessageEvidence, MessageGuid, MessageTimestamp,
    MessagesDataSource, NativeBatch, ParticipantId, PausedChat, RawChat, RawMessage, TapbackKind,
};
