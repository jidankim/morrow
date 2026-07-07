use std::fmt;

use super::super::provider_contract::ProviderContractError;
use morrow_detection::ListIntakeValidationError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexProviderError {
    EvidenceTooLarge,
    WorkspaceUnavailable,
    OutputUnavailable,
    MissingCli,
    Timeout,
    CommandFailed,
    InvalidResponse { reason: &'static str },
}

impl fmt::Display for CodexProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EvidenceTooLarge => write!(formatter, "codex provider evidence is too large"),
            Self::WorkspaceUnavailable => write!(formatter, "codex provider workspace unavailable"),
            Self::OutputUnavailable => write!(formatter, "codex provider output unavailable"),
            Self::MissingCli => write!(formatter, "codex provider command is unavailable"),
            Self::Timeout => write!(formatter, "codex provider command timed out"),
            Self::CommandFailed => write!(formatter, "codex provider command failed"),
            Self::InvalidResponse { reason } => {
                write!(formatter, "codex provider response rejected: {reason}")
            }
        }
    }
}

impl std::error::Error for CodexProviderError {}

impl From<ProviderContractError> for CodexProviderError {
    fn from(error: ProviderContractError) -> Self {
        match error {
            ProviderContractError::EvidenceSerialization => Self::InvalidResponse {
                reason: "evidence serialization failed",
            },
            ProviderContractError::EvidenceTooLarge => Self::EvidenceTooLarge,
            ProviderContractError::InvalidCandidate { reason } => Self::InvalidResponse { reason },
        }
    }
}

impl From<ListIntakeValidationError> for CodexProviderError {
    fn from(error: ListIntakeValidationError) -> Self {
        match error {
            ListIntakeValidationError::MalformedJson => Self::InvalidResponse {
                reason: "list-intake provider output malformed",
            },
            ListIntakeValidationError::InvalidProviderOutput { reason } => {
                Self::InvalidResponse { reason }
            }
        }
    }
}
