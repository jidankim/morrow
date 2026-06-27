use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    Sqlite { message: String },
    InvalidInput { field: &'static str, reason: String },
    PrivacyViolation { reason: String },
    InvalidTransition { from: String, to: String },
    CandidateMissing { id: String },
    ExternalMappingConflict { external_object_id: String },
}

impl Display for StorageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Sqlite { message } => write!(f, "sqlite error: {message}"),
            Self::InvalidInput { field, reason } => write!(f, "invalid {field}: {reason}"),
            Self::PrivacyViolation { reason } => write!(f, "privacy violation: {reason}"),
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid candidate transition from {from} to {to}")
            }
            Self::CandidateMissing { id } => write!(f, "candidate not found: {id}"),
            Self::ExternalMappingConflict { external_object_id } => {
                write!(f, "external object already mapped: {external_object_id}")
            }
        }
    }
}

impl std::error::Error for StorageError {}

impl From<std::io::Error> for StorageError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
