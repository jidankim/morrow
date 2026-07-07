use morrow_messages::MessageEvidence;

use crate::list_intake::{ListIntakeProfile, ValidatedListIntakeExtraction};
use crate::types::ProviderIdentity;

pub trait AiProvider {
    fn extract(&self, request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError>;

    fn extract_list_intake(
        &self,
        _request: ListIntakeProviderRequest<'_>,
    ) -> Result<ValidatedListIntakeExtraction, ProviderError> {
        Err(ProviderError::Unavailable {
            reason: "list-intake provider is not configured".to_owned(),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ProviderRequest<'a> {
    evidence: &'a [MessageEvidence],
    identity: &'a ProviderIdentity,
    reference_timezone: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct ListIntakeProviderRequest<'a> {
    profile: &'a ListIntakeProfile,
    evidence: &'a MessageEvidence,
    reference_timezone: &'a str,
}

impl<'a> ListIntakeProviderRequest<'a> {
    pub fn new(
        profile: &'a ListIntakeProfile,
        evidence: &'a MessageEvidence,
        reference_timezone: &'a str,
    ) -> Self {
        Self {
            profile,
            evidence,
            reference_timezone,
        }
    }

    pub const fn profile(&self) -> &'a ListIntakeProfile {
        self.profile
    }

    pub const fn evidence(&self) -> &'a MessageEvidence {
        self.evidence
    }

    pub const fn reference_timezone(&self) -> &'a str {
        self.reference_timezone
    }
}

impl<'a> ProviderRequest<'a> {
    pub(crate) const fn new(
        evidence: &'a [MessageEvidence],
        identity: &'a ProviderIdentity,
        reference_timezone: &'a str,
    ) -> Self {
        Self {
            evidence,
            identity,
            reference_timezone,
        }
    }

    pub const fn evidence(&self) -> &'a [MessageEvidence] {
        self.evidence
    }

    pub const fn identity(&self) -> &'a ProviderIdentity {
        self.identity
    }

    pub const fn reference_timezone(&self) -> &'a str {
        self.reference_timezone
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderResponse {
    raw_json: String,
}

impl ProviderResponse {
    pub fn new(raw_json: &str) -> Self {
        Self {
            raw_json: raw_json.to_owned(),
        }
    }

    pub fn raw_json(&self) -> &str {
        &self.raw_json
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProviderError {
    #[error("provider unavailable: {reason}")]
    Unavailable { reason: String },
}
