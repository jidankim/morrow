mod candidate;
mod payload;

pub(crate) use candidate::{candidate_schema, validate_candidate_json};
pub(crate) use payload::evidence_payload_text;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderContractError {
    EvidenceSerialization,
    EvidenceTooLarge,
    InvalidCandidate { reason: &'static str },
}
