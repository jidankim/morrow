use morrow_storage::{
    CandidateKind, ExternalObjectMapping, ExternalSource, QueuedProposal, ReminderProposalPayload,
    Store,
};
use time::{
    format_description::well_known::{Iso8601, Rfc3339},
    OffsetDateTime, PrimitiveDateTime,
};
use time_tz::{timezones, OffsetResult, PrimitiveDateTimeExt};

use crate::native_bridge::eventkit_proposal::{
    ProposedReminder, ReminderDate, ReminderDue, ReminderProposalMetadata, ReminderTime,
};

use super::{
    external_proposal_error, storage_error, ProposalReplayAdapter, ScanSelectedChatsError,
    MAPPED_AT,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReminderProposalReceipt {
    pub reminder_id: String,
    pub source_id: String,
}

pub(super) fn reminder_mapping_from_store(
    store: &Store,
    candidate: &QueuedProposal,
    adapter: &impl ProposalReplayAdapter,
) -> Result<ExternalObjectMapping, ScanSelectedChatsError> {
    let payloads = store
        .reminder_proposal_payloads(std::slice::from_ref(&candidate.candidate_id))
        .map_err(storage_error)?;
    let Some(payload) = payloads.into_iter().next() else {
        return Err(external_proposal_error(format!(
            "reminder proposal payload unavailable for {}",
            candidate.candidate_id.as_str()
        )));
    };
    reminder_mapping_from_payload(&payload, adapter)
}

pub(super) fn reminder_mapping_from_payload(
    payload: &ReminderProposalPayload,
    adapter: &impl ProposalReplayAdapter,
) -> Result<ExternalObjectMapping, ScanSelectedChatsError> {
    let reminder = proposed_reminder_from_payload(payload)?;
    let receipt = adapter.create_reminder_proposal(reminder)?;
    Ok(ExternalObjectMapping {
        candidate_id: payload.candidate_id.clone(),
        source: ExternalSource::Reminders,
        external_object_id: receipt.reminder_id,
        external_source_id: receipt.source_id,
        mapped_at: MAPPED_AT,
    })
}

pub(super) fn proposed_reminder_from_payload(
    payload: &ReminderProposalPayload,
) -> Result<ProposedReminder, ScanSelectedChatsError> {
    match payload.kind {
        CandidateKind::TaskReminder => {}
        CandidateKind::CalendarEvent
        | CandidateKind::EventUpdate
        | CandidateKind::EventReschedule
        | CandidateKind::EventCancellation
        | CandidateKind::ReminderUpdate
        | CandidateKind::ReminderReschedule
        | CandidateKind::ReminderCancellation => {
            return Err(external_proposal_error(format!(
                "reminder payload has unsupported kind {}",
                payload.kind.as_str()
            )));
        }
    }
    Ok(ProposedReminder {
        title: payload.title.clone(),
        due: parse_normalized_reminder_due(&payload.normalized_time)?,
        metadata: ReminderProposalMetadata {
            candidate_id: payload.candidate_id.as_str().to_owned(),
            source_id: payload.source_id.clone(),
        },
    })
}

fn parse_normalized_reminder_due(value: &str) -> Result<ReminderDue, ScanSelectedChatsError> {
    match value.split_once('[') {
        Some((local_time, timezone_with_bracket)) => {
            let timezone_name = timezone_with_bracket
                .strip_suffix(']')
                .ok_or_else(|| external_proposal_error("invalid normalized_time timezone"))?;
            let local_datetime =
                PrimitiveDateTime::parse(local_time, &Iso8601::DEFAULT).map_err(|error| {
                    external_proposal_error(format!("invalid normalized_time: {error}"))
                })?;
            let timezone = timezones::get_by_name(timezone_name)
                .ok_or_else(|| external_proposal_error("unsupported normalized_time timezone"))?;
            match local_datetime.assume_timezone(timezone) {
                OffsetResult::Some(_) => Ok(reminder_due_from_local(
                    local_datetime,
                    Some(timezone_name.to_owned()),
                )),
                OffsetResult::Ambiguous(_, _) | OffsetResult::None => Err(external_proposal_error(
                    "ambiguous or invalid normalized_time timezone",
                )),
            }
        }
        None => {
            let datetime = OffsetDateTime::parse(value, &Rfc3339).map_err(|error| {
                external_proposal_error(format!("invalid normalized_time: {error}"))
            })?;
            Ok(reminder_due_from_offset(datetime))
        }
    }
}

fn reminder_due_from_local(
    datetime: PrimitiveDateTime,
    timezone_name: Option<String>,
) -> ReminderDue {
    ReminderDue {
        date: ReminderDate {
            year: datetime.year(),
            month: u8::from(datetime.month()),
            day: datetime.day(),
        },
        time: Some(ReminderTime {
            hour: datetime.hour(),
            minute: datetime.minute(),
        }),
        timezone_name,
    }
}

fn reminder_due_from_offset(datetime: OffsetDateTime) -> ReminderDue {
    ReminderDue {
        date: ReminderDate {
            year: datetime.year(),
            month: u8::from(datetime.month()),
            day: datetime.day(),
        },
        time: Some(ReminderTime {
            hour: datetime.hour(),
            minute: datetime.minute(),
        }),
        timezone_name: Some("UTC".to_owned()),
    }
}
