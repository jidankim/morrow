use morrow_messages::MessageEvidence;

use crate::types::ProviderIdentity;

pub trait AiProvider {
    fn extract(&self, request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError>;
}

#[derive(Debug, Clone, Copy)]
pub struct ProviderRequest<'a> {
    evidence: &'a [MessageEvidence],
    identity: &'a ProviderIdentity,
}

impl<'a> ProviderRequest<'a> {
    pub(crate) const fn new(
        evidence: &'a [MessageEvidence],
        identity: &'a ProviderIdentity,
    ) -> Self {
        Self { evidence, identity }
    }

    pub const fn evidence(&self) -> &'a [MessageEvidence] {
        self.evidence
    }

    pub const fn identity(&self) -> &'a ProviderIdentity {
        self.identity
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
