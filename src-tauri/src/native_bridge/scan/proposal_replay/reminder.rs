use morrow_calendar::{CalendarSourceId, CandidateId};
use morrow_storage::{
    CandidateKind, ExternalObjectMapping, ExternalSource, QueuedProposal, ReminderProposalPayload,
    Store,
};

use crate::native_bridge::eventkit_proposal::{
    ReminderDueComponents as EventKitReminderDueComponents,
    ReminderDueTimeZone as EventKitReminderDueTimeZone, ReminderProposalRecord,
};

use super::{
    external_proposal_error,
    normalized_time::{parse_normalized_time, ReminderDueComponents, ReminderDueTimeZone},
    storage_error, ProposalReplayAdapter, ScanSelectedChatsError, MAPPED_AT,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReminderProposalReceipt {
    pub reminder_id: String,
    pub list_id: String,
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
        external_source_id: receipt.list_id,
        mapped_at: MAPPED_AT,
    })
}

pub(super) fn proposed_reminder_from_payload(
    payload: &ReminderProposalPayload,
) -> Result<ReminderProposalRecord, ScanSelectedChatsError> {
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
    Ok(ReminderProposalRecord {
        candidate_id: CandidateId::new(payload.candidate_id.as_str())
            .map_err(|error| external_proposal_error(error.to_string()))?,
        selected_source_id: CalendarSourceId::new(&payload.source_id)
            .map_err(|error| external_proposal_error(error.to_string()))?,
        title: payload.title.clone(),
        notes: String::new(),
        due: reminder_due_from_normalized_time(&payload.normalized_time)?,
    })
}

fn reminder_due_from_normalized_time(
    value: &str,
) -> Result<EventKitReminderDueComponents, ScanSelectedChatsError> {
    let parsed = parse_normalized_time(value)?;
    Ok(reminder_due_from_components(parsed.reminder_due_components))
}

fn reminder_due_from_components(
    components: ReminderDueComponents,
) -> EventKitReminderDueComponents {
    EventKitReminderDueComponents {
        year: components.year,
        month: components.month,
        day: components.day,
        hour: components.hour,
        minute: components.minute,
        second: components.second,
        time_zone: match components.time_zone {
            ReminderDueTimeZone::Named(name) => EventKitReminderDueTimeZone::Named(name),
            ReminderDueTimeZone::Utc => EventKitReminderDueTimeZone::Utc,
        },
    }
}
