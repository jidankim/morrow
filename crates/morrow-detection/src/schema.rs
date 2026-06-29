use morrow_messages::MessageEvidence;
use morrow_storage::CandidateKind;
use serde::Deserialize;

use crate::parser::ParsedCandidate;
use crate::types::{CivilDateTime, DetectionConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderCandidate {
    pub parsed: ParsedCandidate,
    pub title: String,
    pub normalized_time: String,
    pub anchor_message_guid: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SchemaRejection {
    InvalidJson,
    InvalidSchema,
    HallucinatedEvidence,
    ParserConflict,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderCandidatePayload {
    kind: String,
    title: String,
    confidence_millis: i64,
    normalized_time: String,
    anchor_message_guid: String,
    evidence_message_guids: Vec<String>,
}

pub(crate) fn parse_provider_candidate(
    raw_json: &str,
    evidence: &[MessageEvidence],
    parser_time: Option<CivilDateTime>,
    config: &DetectionConfig,
) -> Result<ProviderCandidate, SchemaRejection> {
    let payload: ProviderCandidatePayload = serde_json::from_str(raw_json).map_err(|err| {
        if err.is_syntax() || err.is_eof() {
            SchemaRejection::InvalidJson
        } else {
            SchemaRejection::InvalidSchema
        }
    })?;
    let kind = parse_kind(&payload.kind)?;
    if payload.title.is_empty()
        || payload.title.len() > 160
        || payload.anchor_message_guid.is_empty()
        || payload.evidence_message_guids.is_empty()
        || payload.confidence_millis < 0
        || payload.confidence_millis > 1000
    {
        return Err(SchemaRejection::InvalidSchema);
    }
    let provider_time = CivilDateTime::parse_normalized(&payload.normalized_time)
        .map_err(|_| SchemaRejection::InvalidSchema)?;
    if provider_time <= config.reference.observed {
        return Err(SchemaRejection::InvalidSchema);
    }
    if !evidence_contains(evidence, &payload.anchor_message_guid)
        || payload
            .evidence_message_guids
            .iter()
            .any(|guid| !evidence_contains(evidence, guid))
    {
        return Err(SchemaRejection::HallucinatedEvidence);
    }
    if parser_time.is_some_and(|time| time != provider_time) {
        return Err(SchemaRejection::ParserConflict);
    }
    let normalized_time = provider_normalized_time(&payload.normalized_time, provider_time, config)?;
    Ok(ProviderCandidate {
        parsed: ParsedCandidate {
            kind,
            time: provider_time,
            confidence_millis: payload.confidence_millis,
        },
        title: payload.title,
        normalized_time,
        anchor_message_guid: payload.anchor_message_guid,
    })
}

fn provider_normalized_time(
    raw: &str,
    provider_time: CivilDateTime,
    config: &DetectionConfig,
) -> Result<String, SchemaRejection> {
    if raw.contains('[') {
        if raw.ends_with(']') {
            return Ok(raw.to_owned());
        }
        return Err(SchemaRejection::InvalidSchema);
    }
    if raw.ends_with('Z') {
        return Ok(raw.to_owned());
    }
    Ok(provider_time.normalized(&config.reference.timezone))
}

fn parse_kind(raw: &str) -> Result<CandidateKind, SchemaRejection> {
    match raw {
        "calendar_event" => Ok(CandidateKind::CalendarEvent),
        "task_reminder" => Ok(CandidateKind::TaskReminder),
        "event_update" => Ok(CandidateKind::EventUpdate),
        "event_reschedule" => Ok(CandidateKind::EventReschedule),
        "event_cancellation" => Ok(CandidateKind::EventCancellation),
        "reminder_update" => Ok(CandidateKind::ReminderUpdate),
        "reminder_reschedule" => Ok(CandidateKind::ReminderReschedule),
        "reminder_cancellation" => Ok(CandidateKind::ReminderCancellation),
        _ => Err(SchemaRejection::InvalidSchema),
    }
}

fn evidence_contains(evidence: &[MessageEvidence], message_guid: &str) -> bool {
    evidence
        .iter()
        .any(|message| message.message_guid.as_str() == message_guid)
}
