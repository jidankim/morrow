use morrow_reminders::{ReminderDate, ReminderDraft, ReminderTime, SourceId};
use morrow_storage::{
    CandidateKind, ExternalObjectMapping, ExternalSource, QueuedProposal, ReminderProposalPayload,
    Store,
};

use super::{
    external_proposal_error,
    normalized_time::{parse_normalized_time, ReminderDueComponents},
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
    let due_components = reminder_due_from_normalized_time(&payload.normalized_time)?;
    let reminder = proposed_reminder_from_payload(payload)?;
    let source_id = SourceId::parse(&payload.source_id)
        .map_err(|error| external_proposal_error(error.to_string()))?;
    let receipt = adapter.create_reminder_proposal(
        &payload.candidate_id,
        source_id,
        due_components,
        reminder,
    )?;
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
) -> Result<ReminderDraft, ScanSelectedChatsError> {
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
    let components = reminder_due_from_normalized_time(&payload.normalized_time)?;
    let due_date = ReminderDate::parse(&format!(
        "{:04}-{:02}-{:02}",
        components.year, components.month, components.day
    ))
    .map_err(|error| external_proposal_error(error.to_string()))?;
    let due_time = ReminderTime::parse(&format!("{:02}:{:02}", components.hour, components.minute))
        .map(Some)
        .map_err(|error| external_proposal_error(error.to_string()))?;
    ReminderDraft::new(&payload.title, due_date, due_time)
        .map_err(|error| external_proposal_error(error.to_string()))
}

fn reminder_due_from_normalized_time(
    value: &str,
) -> Result<ReminderDueComponents, ScanSelectedChatsError> {
    let parsed = parse_normalized_time(value)?;
    Ok(parsed.reminder_due_components)
}
