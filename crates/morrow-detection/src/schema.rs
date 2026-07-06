use morrow_messages::MessageEvidence;
use morrow_storage::{validate_normalized_time as storage_validate_normalized_time, CandidateKind};
use serde::Deserialize;

use crate::parser::ParsedCandidate;
use crate::types::{CivilDateTime, DetectionConfig};

mod list_reminder;

use list_reminder::{
    default_list_reminder_due_time, rendered_list_title, validate_list_item_evidence,
    ListReminderItem,
};

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

impl SchemaRejection {
    pub(crate) const fn reason(self) -> &'static str {
        match self {
            Self::InvalidJson => "provider_invalid_json",
            Self::InvalidSchema => "provider_schema_rejected",
            Self::HallucinatedEvidence => "provider_hallucinated_evidence",
            Self::ParserConflict => "parser_provider_time_conflict",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderCandidatePayload {
    kind: String,
    title: String,
    confidence_millis: i64,
    normalized_time: String,
    #[serde(default)]
    anchor_message_guid: Option<String>,
    #[serde(default)]
    evidence_message_guids: Option<Vec<String>>,
    #[serde(default)]
    anchor_evidence_id: Option<String>,
    #[serde(default)]
    evidence_ids: Option<Vec<String>>,
    #[serde(default)]
    items: Option<Vec<ListReminderItem>>,
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
        || payload.confidence_millis < 0
        || payload.confidence_millis > 1000
    {
        return Err(SchemaRejection::InvalidSchema);
    }
    let (anchor_message_guid, _evidence_message_guids) =
        provider_evidence_guids(&payload, evidence)?;
    validate_list_item_evidence(payload.items.as_deref(), evidence)?;
    let locally_computed_time = default_list_reminder_due_time(&payload, config);
    let title = match payload.items.as_deref() {
        Some(items) => rendered_list_title(kind, items)?,
        None => payload.title,
    };
    let (provider_time, normalized_time) = match locally_computed_time {
        Some(time) => (time, time.normalized(&config.reference.timezone)),
        None => {
            storage_validate_normalized_time(&payload.normalized_time)
                .map_err(|_| SchemaRejection::InvalidSchema)?;
            let provider_time = CivilDateTime::parse_normalized(&payload.normalized_time)
                .map_err(|_| SchemaRejection::InvalidSchema)?;
            if provider_time <= config.reference.observed {
                return Err(SchemaRejection::InvalidSchema);
            }
            if parser_time.is_some_and(|time| time != provider_time) {
                return Err(SchemaRejection::ParserConflict);
            }
            (provider_time, payload.normalized_time)
        }
    };
    Ok(ProviderCandidate {
        parsed: ParsedCandidate {
            kind,
            time: provider_time,
            confidence_millis: payload.confidence_millis,
            title_source: None,
        },
        title,
        normalized_time,
        anchor_message_guid,
    })
}

fn provider_evidence_guids(
    payload: &ProviderCandidatePayload,
    evidence: &[MessageEvidence],
) -> Result<(String, Vec<String>), SchemaRejection> {
    match (
        &payload.anchor_message_guid,
        &payload.evidence_message_guids,
        &payload.anchor_evidence_id,
        &payload.evidence_ids,
    ) {
        (Some(anchor), Some(evidence_guids), None, None) => {
            if anchor.is_empty() || evidence_guids.is_empty() {
                return Err(SchemaRejection::InvalidSchema);
            }
            if !evidence_contains(evidence, anchor)
                || evidence_guids
                    .iter()
                    .any(|guid| !evidence_contains(evidence, guid))
            {
                return Err(SchemaRejection::HallucinatedEvidence);
            }
            Ok((anchor.clone(), evidence_guids.clone()))
        }
        (None, None, Some(anchor_id), Some(evidence_ids)) => {
            if anchor_id.is_empty() || evidence_ids.is_empty() {
                return Err(SchemaRejection::InvalidSchema);
            }
            let anchor = evidence_guid_for_id(evidence, anchor_id)?;
            let evidence_guids = evidence_ids
                .iter()
                .map(|id| evidence_guid_for_id(evidence, id))
                .collect::<Result<Vec<_>, _>>()?;
            Ok((anchor, evidence_guids))
        }
        _ => Err(SchemaRejection::InvalidSchema),
    }
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

fn evidence_guid_for_id(
    evidence: &[MessageEvidence],
    evidence_id: &str,
) -> Result<String, SchemaRejection> {
    let index_text = evidence_id
        .strip_prefix("evidence://selected/")
        .ok_or(SchemaRejection::HallucinatedEvidence)?;
    let index = index_text
        .parse::<usize>()
        .map_err(|_| SchemaRejection::HallucinatedEvidence)?;
    evidence
        .get(index)
        .map(|message| message.message_guid.as_str().to_owned())
        .ok_or(SchemaRejection::HallucinatedEvidence)
}
