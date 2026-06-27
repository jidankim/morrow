use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessagesError {
    PermissionDenied,
    InvalidInput { field: &'static str, reason: String },
    NativeUnavailable { reason: String },
}

impl Display for MessagesError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PermissionDenied => write!(f, "messages permission denied"),
            Self::InvalidInput { field, reason } => {
                write!(f, "invalid {field}: {reason}")
            }
            Self::NativeUnavailable { reason } => write!(f, "messages unavailable: {reason}"),
        }
    }
}

impl std::error::Error for MessagesError {}
