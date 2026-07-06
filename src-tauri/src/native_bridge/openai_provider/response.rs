use morrow_detection::ProviderResponse;
use morrow_messages::MessageEvidence;
use serde::Deserialize;
use serde_json::Value;

use super::{super::provider_contract, invalid_response, OpenAiHttpResponse, OpenAiProviderError};

#[derive(Debug, Deserialize)]
struct ResponsesEnvelope {
    status: String,
    #[serde(default)]
    incomplete_details: Option<Value>,
    #[serde(default)]
    output: Vec<ResponseOutput>,
}

#[derive(Debug, Deserialize)]
struct ResponseOutput {
    #[serde(rename = "type")]
    item_type: String,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    content: Vec<ResponseContent>,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: Option<String>,
}

pub(super) fn response_candidate(
    response: OpenAiHttpResponse,
    evidence: &[MessageEvidence],
) -> Result<ProviderResponse, OpenAiProviderError> {
    if !(200..=299).contains(&response.status_code) {
        return Err(OpenAiProviderError::HttpStatus {
            status_code: response.status_code,
        });
    }
    let envelope: ResponsesEnvelope = serde_json::from_value(response.body)
        .map_err(|_| invalid_response("invalid response envelope"))?;
    if envelope.status != "completed" {
        return Err(invalid_response("response was not completed"));
    }
    if envelope
        .incomplete_details
        .as_ref()
        .is_some_and(|details| !details.is_null())
    {
        return Err(invalid_response("response included incomplete details"));
    }
    let text = output_text(&envelope)?;
    let localized = local_candidate_text(&text, evidence)?;
    Ok(ProviderResponse::new(&localized))
}

fn output_text(envelope: &ResponsesEnvelope) -> Result<String, OpenAiProviderError> {
    let mut candidate_text: Option<String> = None;
    for output in &envelope.output {
        if output.item_type != "message" || output.role.as_deref() != Some("assistant") {
            continue;
        }
        if output.status.as_deref() != Some("completed") {
            return Err(invalid_response("assistant message was not completed"));
        }
        for content in &output.content {
            match content.content_type.as_str() {
                "refusal" => return Err(invalid_response("assistant refused")),
                "output_text" if candidate_text.is_some() => {
                    return Err(invalid_response("multiple output_text candidates"));
                }
                "output_text" => candidate_text = content.text.clone(),
                _ => {}
            }
        }
    }
    candidate_text.ok_or_else(|| invalid_response("missing output_text"))
}

fn local_candidate_text(
    text: &str,
    evidence: &[MessageEvidence],
) -> Result<String, OpenAiProviderError> {
    provider_contract::localize_candidate_json(text, evidence).map_err(|error| match error {
        provider_contract::ProviderContractError::EvidenceSerialization => {
            invalid_response("evidence serialization failed")
        }
        provider_contract::ProviderContractError::EvidenceTooLarge => {
            OpenAiProviderError::EvidenceTooLarge
        }
        provider_contract::ProviderContractError::InvalidCandidate { reason } => {
            invalid_response(reason)
        }
    })
}
