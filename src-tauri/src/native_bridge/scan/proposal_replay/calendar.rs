use morrow_calendar::{
    CalendarSourceId, CandidateId as CalendarCandidateId, ProposalMetadata, ProposedEvent,
    TimeRange,
};
use morrow_storage::{
    CalendarProposalPayload, CandidateKind, ExternalObjectMapping, ExternalSource, QueuedProposal,
    Store,
};
use time::{format_description::well_known::Rfc3339, Date, Month, OffsetDateTime};

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
    let rfc3339 = match value.split_once('[') {
        Some((local_time, timezone_with_bracket)) => {
            let timezone = timezone_with_bracket
                .strip_suffix(']')
                .ok_or_else(|| external_proposal_error("invalid normalized_time timezone"))?;
            let local_datetime = OffsetDateTime::parse(&format!("{local_time}Z"), &Rfc3339)
                .map_err(|error| {
                    external_proposal_error(format!("invalid normalized_time: {error}"))
                })?;
            format!(
                "{local_time}{}",
                timezone_offset_designator(local_datetime, timezone)?
            )
        }
        None => value.to_owned(),
    };
    OffsetDateTime::parse(&rfc3339, &Rfc3339)
        .map(OffsetDateTime::unix_timestamp)
        .map_err(|error| external_proposal_error(format!("invalid normalized_time: {error}")))
}

fn timezone_offset_designator(
    local_datetime: OffsetDateTime,
    timezone: &str,
) -> Result<&'static str, ScanSelectedChatsError> {
    match timezone {
        "UTC" | "Etc/UTC" => Ok("Z"),
        "Asia/Seoul" => Ok("+09:00"),
        "America/New_York" => {
            if is_new_york_daylight_time(local_datetime)? {
                Ok("-04:00")
            } else {
                Ok("-05:00")
            }
        }
        "Europe/London" => {
            if is_london_summer_time(local_datetime)? {
                Ok("+01:00")
            } else {
                Ok("Z")
            }
        }
        _other => Err(external_proposal_error(
            "unsupported normalized_time timezone",
        )),
    }
}

fn is_new_york_daylight_time(
    local_datetime: OffsetDateTime,
) -> Result<bool, ScanSelectedChatsError> {
    let day = local_datetime.day();
    let hour = local_datetime.hour();
    match local_datetime.month() {
        Month::April
        | Month::May
        | Month::June
        | Month::July
        | Month::August
        | Month::September
        | Month::October => Ok(true),
        Month::January | Month::February | Month::December => Ok(false),
        Month::March => {
            let boundary = nth_sunday(local_datetime.year(), Month::March, 2)?;
            Ok(day > boundary || (day == boundary && hour >= 3))
        }
        Month::November => {
            let boundary = nth_sunday(local_datetime.year(), Month::November, 1)?;
            Ok(day < boundary || (day == boundary && hour < 2))
        }
    }
}

fn is_london_summer_time(local_datetime: OffsetDateTime) -> Result<bool, ScanSelectedChatsError> {
    let day = local_datetime.day();
    let hour = local_datetime.hour();
    match local_datetime.month() {
        Month::April
        | Month::May
        | Month::June
        | Month::July
        | Month::August
        | Month::September => Ok(true),
        Month::January | Month::February | Month::November | Month::December => Ok(false),
        Month::March => {
            let boundary = last_sunday(local_datetime.year(), Month::March)?;
            Ok(day > boundary || (day == boundary && hour >= 2))
        }
        Month::October => {
            let boundary = last_sunday(local_datetime.year(), Month::October)?;
            Ok(day < boundary || (day == boundary && hour < 2))
        }
    }
}

fn nth_sunday(year: i32, month: Month, ordinal: u8) -> Result<u8, ScanSelectedChatsError> {
    let first = Date::from_calendar_date(year, month, 1).map_err(time_error)?;
    let offset = (7 - first.weekday().number_days_from_sunday()) % 7;
    Ok(1 + offset + 7 * (ordinal - 1))
}

fn last_sunday(year: i32, month: Month) -> Result<u8, ScanSelectedChatsError> {
    let last_day = month.length(year);
    let last = Date::from_calendar_date(year, month, last_day).map_err(time_error)?;
    Ok(last_day - last.weekday().number_days_from_sunday())
}

fn time_error(error: time::error::ComponentRange) -> ScanSelectedChatsError {
    external_proposal_error(format!("invalid normalized_time date: {error}"))
}

fn calendar_error(error: morrow_calendar::CalendarError) -> ScanSelectedChatsError {
    external_proposal_error(error.to_string())
}
