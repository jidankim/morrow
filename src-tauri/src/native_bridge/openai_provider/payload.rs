use morrow_messages::MessageEvidence;
use serde_json::{json, Value};

use super::{super::provider_contract, OpenAiProviderError, OPENAI_MODEL};

pub(super) fn request_body(evidence: &[MessageEvidence]) -> Result<Value, OpenAiProviderError> {
    let evidence_text =
        provider_contract::evidence_payload_text(evidence).map_err(|error| match error {
            provider_contract::ProviderContractError::EvidenceSerialization => {
                OpenAiProviderError::InvalidResponse {
                    reason: "evidence serialization failed",
                }
            }
            provider_contract::ProviderContractError::EvidenceTooLarge => {
                OpenAiProviderError::EvidenceTooLarge
            }
            provider_contract::ProviderContractError::InvalidCandidate { reason } => {
                OpenAiProviderError::InvalidResponse { reason }
            }
        })?;
    Ok(json!({
        "model": OPENAI_MODEL,
        "input": [{
            "role": "user",
            "content": [{ "type": "input_text", "text": evidence_text }]
        }],
        "text": {
            "format": {
                "type": "json_schema",
                "name": "morrow_provider_candidate",
                "strict": true,
                "schema": candidate_schema()
            }
        }
    }))
}

fn candidate_schema() -> Value {
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
