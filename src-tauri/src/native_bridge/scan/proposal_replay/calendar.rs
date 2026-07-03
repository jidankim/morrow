use morrow_calendar::{
    CalendarSourceId, CandidateId as CalendarCandidateId, ProposalMetadata, ProposedEvent,
    TimeRange,
};
use morrow_storage::{
    CalendarProposalPayload, CandidateKind, ExternalObjectMapping, ExternalSource, QueuedProposal,
    Store,
};
use time::{
    format_description::well_known::{Iso8601, Rfc3339},
    OffsetDateTime, PrimitiveDateTime,
};
use time_tz::{timezones, OffsetResult, PrimitiveDateTimeExt};

use super::{
    external_proposal_error, storage_error, ProposalReplayAdapter, ScanSelectedChatsError,
    DEFAULT_CALENDAR_EVENT_DURATION_SECONDS, MAPPED_AT,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarProposalReceipt {
    pub event_id: String,
    pub source_id: String,
}

pub(super) fn calendar_mapping_from_store(
    store: &Store,
    candidate: &QueuedProposal,
    adapter: &impl ProposalReplayAdapter,
) -> Result<ExternalObjectMapping, ScanSelectedChatsError> {
    let payloads = store
        .calendar_proposal_payloads(std::slice::from_ref(&candidate.candidate_id))
        .map_err(storage_error)?;
    let Some(payload) = payloads.into_iter().next() else {
        return Err(external_proposal_error(format!(
            "calendar proposal payload unavailable for {}",
            candidate.candidate_id.as_str()
        )));
    };
    calendar_mapping_from_payload(&payload, adapter)
}

pub(super) fn calendar_mapping_from_payload(
    payload: &CalendarProposalPayload,
    adapter: &impl ProposalReplayAdapter,
) -> Result<ExternalObjectMapping, ScanSelectedChatsError> {
    let event = proposed_event_from_payload(payload)?;
    let receipt = adapter.create_calendar_proposal(event)?;
    Ok(ExternalObjectMapping {
        candidate_id: payload.candidate_id.clone(),
        source: ExternalSource::Calendar,
        external_object_id: receipt.event_id,
        external_source_id: receipt.source_id,
        mapped_at: MAPPED_AT,
    })
}

pub(super) fn proposed_event_from_payload(
    payload: &CalendarProposalPayload,
) -> Result<ProposedEvent, ScanSelectedChatsError> {
    match payload.kind {
        CandidateKind::CalendarEvent => {}
        CandidateKind::TaskReminder
        | CandidateKind::EventUpdate
        | CandidateKind::EventReschedule
        | CandidateKind::EventCancellation
        | CandidateKind::ReminderUpdate
        | CandidateKind::ReminderReschedule
        | CandidateKind::ReminderCancellation => {
            return Err(external_proposal_error(format!(
                "calendar payload has unsupported kind {}",
                payload.kind.as_str()
            )));
        }
    }
    let start_unix = parse_normalized_time(&payload.normalized_time)?;
    let end_unix = start_unix
        .checked_add(DEFAULT_CALENDAR_EVENT_DURATION_SECONDS)
        .ok_or_else(|| {
            external_proposal_error("normalized_time is too large for default event duration")
        })?;
    Ok(ProposedEvent {
        title: payload.title.clone(),
        time_range: TimeRange::new(start_unix, end_unix).map_err(calendar_error)?,
        user_note: String::new(),
        video_url: None,
        metadata: ProposalMetadata {
            candidate_id: CalendarCandidateId::new(payload.candidate_id.as_str())
                .map_err(calendar_error)?,
            source_id: CalendarSourceId::new(&payload.source_id).map_err(calendar_error)?,
        },
    })
}

fn parse_normalized_time(value: &str) -> Result<i64, ScanSelectedChatsError> {
    match value.split_once('[') {
        Some((local_time, timezone_with_bracket)) => {
            let timezone = timezone_with_bracket
                .strip_suffix(']')
                .ok_or_else(|| external_proposal_error("invalid normalized_time timezone"))?;
            let local_datetime =
                PrimitiveDateTime::parse(local_time, &Iso8601::DEFAULT).map_err(|error| {
                    external_proposal_error(format!("invalid normalized_time: {error}"))
                })?;
            let timezone = timezones::get_by_name(timezone)
                .ok_or_else(|| external_proposal_error("unsupported normalized_time timezone"))?;
            match local_datetime.assume_timezone(timezone) {
                OffsetResult::Some(datetime) => Ok(datetime.unix_timestamp()),
                OffsetResult::Ambiguous(_, _) | OffsetResult::None => Err(external_proposal_error(
                    "ambiguous or invalid normalized_time timezone",
                )),
            }
        }
        None => OffsetDateTime::parse(value, &Rfc3339)
            .map(OffsetDateTime::unix_timestamp)
            .map_err(|error| external_proposal_error(format!("invalid normalized_time: {error}"))),
    }
}

fn calendar_error(error: morrow_calendar::CalendarError) -> ScanSelectedChatsError {
    external_proposal_error(error.to_string())
}
