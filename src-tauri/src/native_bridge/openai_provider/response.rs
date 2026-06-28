use morrow_detection::ProviderResponse;
use morrow_messages::MessageEvidence;
use morrow_storage::CandidateKind;
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
    anchor_message_guid: String,
    evidence_message_guids: Vec<String>,
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
    validate_candidate_object(&text, evidence)?;
    Ok(ProviderResponse::new(&text))
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

fn validate_candidate_object(
    text: &str,
    evidence: &[MessageEvidence],
) -> Result<(), OpenAiProviderError> {
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
    if candidate.anchor_message_guid.is_empty() || candidate.evidence_message_guids.is_empty() {
        return Err(invalid_response("candidate evidence was invalid"));
    }
    if !evidence_contains(evidence, &candidate.anchor_message_guid)
        || candidate
            .evidence_message_guids
            .iter()
            .any(|guid| !evidence_contains(evidence, guid))
    {
        return Err(invalid_response("candidate evidence was hallucinated"));
    }
    Ok(())
}

fn validate_normalized_time(raw: &str) -> Result<(), OpenAiProviderError> {
    let (date, rest) = raw
        .split_once('T')
        .ok_or_else(|| invalid_response("candidate normalized_time was invalid"))?;
    let time = rest.split_once('[').map_or(rest, |parts| parts.0);
    parse_time(time)?;
    parse_date(date)?;
    Ok(())
}

fn parse_time(raw: &str) -> Result<(u8, u8), OpenAiProviderError> {
    let (hour_text, minute_and_seconds) = raw
        .split_once(':')
        .ok_or_else(|| invalid_response("candidate normalized_time was invalid"))?;
    let hour = hour_text
        .parse::<u8>()
        .map_err(|_| invalid_response("candidate normalized_time was invalid"))?;
    let minute_text = minute_and_seconds
        .split_once(':')
        .map_or(minute_and_seconds, |parts| parts.0);
    let minute = minute_text
        .parse::<u8>()
        .map_err(|_| invalid_response("candidate normalized_time was invalid"))?;
    if hour > 23 || minute > 59 {
        return Err(invalid_response("candidate normalized_time was invalid"));
    }
    Ok((hour, minute))
}

fn parse_date(date: &str) -> Result<(), OpenAiProviderError> {
    let mut parts = date.split('-');
    let year = parse_date_part::<u16>(&mut parts)?;
    let month = parse_date_part::<u8>(&mut parts)?;
    let day = parse_date_part::<u8>(&mut parts)?;
    let Some(max_day) = days_in_month(year, month) else {
        return Err(invalid_response("candidate normalized_time was invalid"));
    };
    if parts.next().is_some() || day == 0 || day > max_day {
        return Err(invalid_response("candidate normalized_time was invalid"));
    }
    Ok(())
}

fn parse_date_part<T: std::str::FromStr>(
    parts: &mut std::str::Split<'_, char>,
) -> Result<T, OpenAiProviderError> {
    parts
        .next()
        .ok_or_else(|| invalid_response("candidate normalized_time was invalid"))?
        .parse::<T>()
        .map_err(|_| invalid_response("candidate normalized_time was invalid"))
}

const fn days_in_month(year: u16, month: u8) -> Option<u8> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 if is_leap_year(year) => Some(29),
        2 => Some(28),
        _ => None,
    }
}

const fn is_leap_year(year: u16) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn evidence_contains(evidence: &[MessageEvidence], message_guid: &str) -> bool {
    evidence
        .iter()
        .any(|message| message.message_guid.as_str() == message_guid)
}
