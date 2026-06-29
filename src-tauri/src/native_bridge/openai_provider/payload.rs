use serde_json::{json, Value};

use morrow_messages::MessageEvidence;

use super::{super::provider_contract, OpenAiProviderError, OPENAI_MODEL};

pub(super) fn request_body(evidence: &[MessageEvidence]) -> Result<Value, OpenAiProviderError> {
    let evidence_text = provider_contract::evidence_payload_text(evidence)?;
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
                "schema": provider_contract::candidate_schema()
            }
        }
    }))
}
