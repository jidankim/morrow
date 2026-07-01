mod candidate;
mod payload;

pub(crate) use candidate::{
    candidate_schema, localize_candidate_json, PROVIDER_CANDIDATE_SCHEMA_VERSION,
};
pub(crate) use payload::evidence_payload_text;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderContractError {
    EvidenceSerialization,
    EvidenceTooLarge,
    InvalidCandidate { reason: &'static str },
}
