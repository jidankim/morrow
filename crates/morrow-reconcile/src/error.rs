use std::fmt::{Display, Formatter};

/// Typed reconciliation failures.
#[derive(Debug)]
pub enum ReconcileError {
    /// A supplied mapping belongs to a different candidate.
    InvalidMappingCandidate {
        /// Candidate expected by the lifecycle input.
        expected: String,
        /// Candidate carried by the mapping.
        actual: String,
    },
    /// An observation field failed boundary parsing.
    InvalidObservation {
        /// Invalid field name.
        field: &'static str,
        /// Human-readable reason.
        reason: String,
    },
    /// Storage rejected an applied lifecycle action.
    Storage(morrow_storage::StorageError),
}

impl Display for ReconcileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMappingCandidate { expected, actual } => {
                write!(
                    f,
                    "mapping candidate mismatch: expected {expected}, got {actual}"
                )
            }
            Self::InvalidObservation { field, reason } => {
                write!(f, "invalid observation {field}: {reason}")
            }
            Self::Storage(err) => write!(f, "storage error: {err}"),
        }
    }
}

impl std::error::Error for ReconcileError {}

impl From<morrow_storage::StorageError> for ReconcileError {
    fn from(value: morrow_storage::StorageError) -> Self {
        Self::Storage(value)
    }
}
