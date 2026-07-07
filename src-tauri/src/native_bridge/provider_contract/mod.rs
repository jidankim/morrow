mod candidate;
mod list_intake;
mod payload;
#[cfg(test)]
mod tests;

use serde_json::{json, Value};

pub(crate) use candidate::{
    candidate_schema, localize_candidate_json, PROVIDER_CANDIDATE_SCHEMA_VERSION,
};
pub(crate) use list_intake::{list_intake_prompt, list_intake_schema};
pub(crate) use payload::evidence_payload_text;

pub(crate) const NORMALIZED_TIME_SCHEMA_DESCRIPTION: &str =
    "Fixed-width normalized timestamp. Use YYYY-MM-DDTHH:MM:SS[Area/Location] for safe \
IANA-style bracketed zones or YYYY-MM-DDTHH:MM:SSZ for UTC. Seconds are mandatory; offsets such \
as +09:00 and variable-width date or time parts are invalid.";
pub(crate) const NORMALIZED_TIME_SCHEMA_PATTERN: &str = r"^(?:\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z|\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\[(?:Africa|America|Antarctica|Arctic|Asia|Atlantic|Australia|Europe|Indian|Pacific|Etc)/[A-Z][A-Za-z0-9_-]*(?:/[A-Z][A-Za-z0-9_-]*)*\])$";

pub(crate) fn normalized_time_schema() -> Value {
    json!({
        "type": "string",
        "description": NORMALIZED_TIME_SCHEMA_DESCRIPTION,
        "pattern": NORMALIZED_TIME_SCHEMA_PATTERN
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderContractError {
    EvidenceSerialization,
    EvidenceTooLarge,
    InvalidCandidate { reason: &'static str },
}
