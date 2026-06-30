use morrow_detection::ProviderResponse;
use morrow_messages::MessageEvidence;
use morrow_storage::{validate_normalized_time as storage_validate_normalized_time, CandidateKind};
use serde::Deserialize;
use serde_json::Value;

use super::{invalid_response, OpenAiHttpResponse, OpenAiProviderError};

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
    let value: Value = serde_json::from_str(text)
        .map_err(|_| invalid_response("candidate text was not valid json"))?;
    if !value.is_object() {
        return Err(invalid_response("candidate text was not an object"));
    }
    let candidate: CandidateObject = serde_json::from_value(value)
        .map_err(|_| invalid_response("candidate object did not match schema"))?;
    CandidateKind::parse(&candidate.kind)
        .map_err(|_| invalid_response("candidate kind was invalid"))?;
    if candidate.title.is_empty() || candidate.title.len() > 160 {
        return Err(invalid_response("candidate title was invalid"));
    }
    if candidate.confidence_millis < 0 || candidate.confidence_millis > 1000 {
        return Err(invalid_response("candidate confidence was invalid"));
    }
    validate_normalized_time(&candidate.normalized_time)?;
    if candidate.anchor_evidence_id.is_empty() || candidate.evidence_ids.is_empty() {
        return Err(invalid_response("candidate evidence was invalid"));
    }
    let anchor_message_guid = evidence_guid_for_id(evidence, &candidate.anchor_evidence_id)?;
    let evidence_message_guids = candidate
        .evidence_ids
        .iter()
        .map(|id| evidence_guid_for_id(evidence, id))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(serde_json::json!({
        "kind": candidate.kind,
        "title": candidate.title,
        "confidence_millis": candidate.confidence_millis,
        "normalized_time": candidate.normalized_time,
        "anchor_message_guid": anchor_message_guid,
        "evidence_message_guids": evidence_message_guids
    })
    .to_string())
}

fn validate_normalized_time(raw: &str) -> Result<(), OpenAiProviderError> {
    storage_validate_normalized_time(raw)
        .map_err(|_| invalid_response("candidate normalized_time was invalid"))
}

fn evidence_guid_for_id(
    evidence: &[MessageEvidence],
    evidence_id: &str,
) -> Result<String, OpenAiProviderError> {
    let index_text = evidence_id
        .strip_prefix("evidence://selected/")
        .ok_or_else(|| invalid_response("candidate evidence was hallucinated"))?;
    let index = index_text
        .parse::<usize>()
        .map_err(|_| invalid_response("candidate evidence was hallucinated"))?;
    evidence
        .get(index)
        .map(|message| message.message_guid.as_str().to_owned())
        .ok_or_else(|| invalid_response("candidate evidence was hallucinated"))
}
