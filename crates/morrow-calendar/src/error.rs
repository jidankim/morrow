use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq, Eq)]
pub enum CalendarError {
    InvalidInput { field: &'static str, reason: String },
    CalendarCreationFailed { source_id: String, reason: String },
    EventCreationFailed { reason: String },
    MetadataMissing,
    MetadataInvalid { reason: String },
}

impl Display for CalendarError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput { field, reason } => write!(f, "invalid {field}: {reason}"),
            Self::CalendarCreationFailed { source_id, reason } => {
                write!(f, "calendar creation failed for {source_id}: {reason}")
            }
            Self::EventCreationFailed { reason } => write!(f, "event creation failed: {reason}"),
            Self::MetadataMissing => write!(f, "morrow metadata block is missing"),
            Self::MetadataInvalid { reason } => write!(f, "morrow metadata is invalid: {reason}"),
        }
    }
}

impl std::error::Error for CalendarError {}
