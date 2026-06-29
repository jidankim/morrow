use morrow_messages::MessageEvidence;
use morrow_storage::CandidateKind;
use serde::Deserialize;
use serde_json::{json, Value};

use super::ProviderContractError;

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
    let (date, rest) = raw
        .split_once('T')
        .ok_or_else(|| invalid_candidate("candidate normalized_time was invalid"))?;
    let time = rest.split_once('[').map_or(rest, |parts| parts.0);
    parse_time(time)?;
    parse_date(date)?;
    Ok(())
}

fn parse_time(raw: &str) -> Result<(u8, u8), ProviderContractError> {
    let (hour_text, minute_and_seconds) = raw
        .split_once(':')
        .ok_or_else(|| invalid_candidate("candidate normalized_time was invalid"))?;
    let hour = hour_text
        .parse::<u8>()
        .map_err(|_| invalid_candidate("candidate normalized_time was invalid"))?;
    let minute_text = minute_and_seconds
        .split_once(':')
        .map_or(minute_and_seconds, |parts| parts.0);
    let minute = minute_text
        .parse::<u8>()
        .map_err(|_| invalid_candidate("candidate normalized_time was invalid"))?;
    if hour > 23 || minute > 59 {
        return Err(invalid_candidate("candidate normalized_time was invalid"));
    }
    Ok((hour, minute))
}

fn parse_date(date: &str) -> Result<(), ProviderContractError> {
    let mut parts = date.split('-');
    let year = parse_date_part::<u16>(&mut parts)?;
    let month = parse_date_part::<u8>(&mut parts)?;
    let day = parse_date_part::<u8>(&mut parts)?;
    let Some(max_day) = days_in_month(year, month) else {
        return Err(invalid_candidate("candidate normalized_time was invalid"));
    };
    if parts.next().is_some() || day == 0 || day > max_day {
        return Err(invalid_candidate("candidate normalized_time was invalid"));
    }
    Ok(())
}

fn parse_date_part<T: std::str::FromStr>(
    parts: &mut std::str::Split<'_, char>,
) -> Result<T, ProviderContractError> {
    parts
        .next()
        .ok_or_else(|| invalid_candidate("candidate normalized_time was invalid"))?
        .parse::<T>()
        .map_err(|_| invalid_candidate("candidate normalized_time was invalid"))
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
