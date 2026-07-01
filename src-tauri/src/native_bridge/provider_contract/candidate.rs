use morrow_messages::MessageEvidence;
use morrow_storage::{validate_normalized_time as storage_validate_normalized_time, CandidateKind};
use serde::Deserialize;
use serde_json::{json, Value};

use super::ProviderContractError;

pub(crate) const PROVIDER_CANDIDATE_SCHEMA_VERSION: &str = "provider-candidate-schema-v1";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateObject {
    kind: String,
    title: String,
    confidence_millis: i64,
    normalized_time: String,
    anchor_evidence_id: String,
    evidence_ids: Vec<String>,
}

pub(crate) fn candidate_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "kind",
            "title",
            "confidence_millis",
            "normalized_time",
            "anchor_evidence_id",
            "evidence_ids"
        ],
        "properties": {
            "kind": {
                "type": "string",
                "enum": [
                    "calendar_event",
                    "task_reminder",
                    "event_update",
                    "event_reschedule",
                    "event_cancellation",
                    "reminder_update",
                    "reminder_reschedule",
                    "reminder_cancellation"
                ]
            },
            "title": { "type": "string" },
            "confidence_millis": { "type": "integer", "minimum": 0, "maximum": 1000 },
            "normalized_time": { "type": "string" },
            "anchor_evidence_id": { "type": "string" },
            "evidence_ids": {
                "type": "array",
                "minItems": 1,
                "items": { "type": "string" }
            }
        }
    })
}

pub(crate) fn localize_candidate_json(
    text: &str,
    evidence: &[MessageEvidence],
) -> Result<String, ProviderContractError> {
    let value: Value = serde_json::from_str(text)
        .map_err(|_| invalid_candidate("candidate text was not valid json"))?;
    if !value.is_object() {
        return Err(invalid_candidate("candidate text was not an object"));
    }
    let candidate: CandidateObject = serde_json::from_value(value)
        .map_err(|_| invalid_candidate("candidate object did not match schema"))?;
    CandidateKind::parse(&candidate.kind)
        .map_err(|_| invalid_candidate("candidate kind was invalid"))?;
    if candidate.title.is_empty() || candidate.title.len() > 160 {
        return Err(invalid_candidate("candidate title was invalid"));
    }
    if candidate.confidence_millis < 0 || candidate.confidence_millis > 1000 {
        return Err(invalid_candidate("candidate confidence was invalid"));
    }
    validate_normalized_time(&candidate.normalized_time)?;
    if candidate.anchor_evidence_id.is_empty() || candidate.evidence_ids.is_empty() {
        return Err(invalid_candidate("candidate evidence was invalid"));
    }
    let anchor_message_guid = evidence_guid_for_id(evidence, &candidate.anchor_evidence_id)?;
    let evidence_message_guids = candidate
        .evidence_ids
        .iter()
        .map(|id| evidence_guid_for_id(evidence, id))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({
        "kind": candidate.kind,
        "title": candidate.title,
        "confidence_millis": candidate.confidence_millis,
        "normalized_time": candidate.normalized_time,
        "anchor_message_guid": anchor_message_guid,
        "evidence_message_guids": evidence_message_guids
    })
    .to_string())
}

fn validate_normalized_time(raw: &str) -> Result<(), ProviderContractError> {
    storage_validate_normalized_time(raw)
        .map_err(|_| invalid_candidate("candidate normalized_time was invalid"))
}

fn evidence_guid_for_id(
    evidence: &[MessageEvidence],
    evidence_id: &str,
) -> Result<String, ProviderContractError> {
    let index_text = evidence_id
        .strip_prefix("evidence://selected/")
        .ok_or_else(|| invalid_candidate("candidate evidence was hallucinated"))?;
    let index = index_text
        .parse::<usize>()
        .map_err(|_| invalid_candidate("candidate evidence was hallucinated"))?;
    evidence
        .get(index)
        .map(|message| message.message_guid.as_str().to_owned())
        .ok_or_else(|| invalid_candidate("candidate evidence was hallucinated"))
}

const fn invalid_candidate(reason: &'static str) -> ProviderContractError {
    ProviderContractError::InvalidCandidate { reason }
}
