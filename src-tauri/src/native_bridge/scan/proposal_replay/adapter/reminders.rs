use std::fmt::{Display, Formatter};

use morrow_calendar::{CalendarSourceId, CandidateId as CalendarCandidateId};
use morrow_reminders::{
    CreatedReminder, ListId, ReminderDate, ReminderDraft, ReminderId, ReminderTime, RemindersError,
    SourceId, MORROW_PROPOSED_LIST_NAME,
};
use morrow_storage::CandidateId as StorageCandidateId;

use crate::native_bridge::eventkit_proposal::{
    EventKitProposalBridge, EventKitReminderProposalError, EventKitReminderProposalReceipt,
    ReminderDueComponents, ReminderDueTimeZone, ReminderProposalRecord,
};

use super::super::normalized_time::{
    ReminderDueComponents as ParsedReminderDueComponents,
    ReminderDueTimeZone as ParsedReminderDueTimeZone,
};

#[derive(Debug, Clone, Copy)]
pub(in crate::native_bridge) struct RemindersProposalBridge<C = EventKitProposalBridge> {
    client: C,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::native_bridge) enum RemindersProposalBridgeError {
    InvalidInput { field: &'static str, reason: String },
    PermissionDenied { reason: String },
    Unavailable { reason: String },
    SaveFailed { reason: String },
}

impl<C> RemindersProposalBridge<C> {
    pub(in crate::native_bridge) const fn new(client: C) -> Self {
        Self { client }
    }
}

impl Default for RemindersProposalBridge<EventKitProposalBridge> {
    fn default() -> Self {
        Self::new(EventKitProposalBridge)
    }
}

impl<C: ReminderProposalClient> RemindersProposalBridge<C> {
    pub(in crate::native_bridge) fn create_proposal(
        &self,
        candidate_id: &StorageCandidateId,
        selected_source_id: SourceId,
        due_components: ParsedReminderDueComponents,
        draft: ReminderDraft,
    ) -> Result<CreatedReminder, RemindersProposalBridgeError> {
        let due_date = draft.due_date;
        let due_time = draft.due_time;
        let record = reminder_record(candidate_id, selected_source_id, due_components, &draft)?;
        let receipt = self.client.propose_reminder(record)?;
        created_reminder_from_receipt(receipt, due_date, due_time)
    }
}

pub(in crate::native_bridge) trait ReminderProposalClient {
    fn propose_reminder(
        &self,
        reminder: ReminderProposalRecord,
    ) -> Result<EventKitReminderProposalReceipt, EventKitReminderProposalError>;
}

impl ReminderProposalClient for EventKitProposalBridge {
    fn propose_reminder(
        &self,
        reminder: ReminderProposalRecord,
    ) -> Result<EventKitReminderProposalReceipt, EventKitReminderProposalError> {
        self.propose_reminder(reminder)
    }
}

impl Display for RemindersProposalBridgeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput { field, reason } => {
                write!(formatter, "invalid Reminders proposal {field}: {reason}")
            }
            Self::PermissionDenied { reason } => {
                write!(formatter, "Reminders permission denied: {reason}")
            }
            Self::Unavailable { reason } => {
                write!(formatter, "Reminders proposal bridge unavailable: {reason}")
            }
            Self::SaveFailed { reason } => {
                write!(formatter, "Reminders proposal save failed: {reason}")
            }
        }
    }
}

impl std::error::Error for RemindersProposalBridgeError {}

fn reminder_record(
    candidate_id: &StorageCandidateId,
    selected_source_id: SourceId,
    due_components: ParsedReminderDueComponents,
    draft: &ReminderDraft,
) -> Result<ReminderProposalRecord, RemindersProposalBridgeError> {
    Ok(ReminderProposalRecord {
        candidate_id: CalendarCandidateId::new(candidate_id.as_str()).map_err(|error| {
            RemindersProposalBridgeError::InvalidInput {
                field: "candidate_id",
                reason: error.to_string(),
            }
        })?,
        selected_source_id: CalendarSourceId::new(selected_source_id.as_str()).map_err(
            |error| RemindersProposalBridgeError::InvalidInput {
                field: "source_id",
                reason: error.to_string(),
            },
        )?,
        title: draft.title.clone(),
        notes: "Created from Messages by Morrow.".to_owned(),
        due: reminder_due_components(due_components),
    })
}

fn reminder_due_components(due: ParsedReminderDueComponents) -> ReminderDueComponents {
    ReminderDueComponents {
        year: due.year,
        month: due.month,
        day: due.day,
        hour: due.hour,
        minute: due.minute,
        second: due.second,
        time_zone: match due.time_zone {
            ParsedReminderDueTimeZone::Named(name) => ReminderDueTimeZone::Named(name),
            ParsedReminderDueTimeZone::Utc => ReminderDueTimeZone::Utc,
        },
    }
}

fn created_reminder_from_receipt(
    receipt: EventKitReminderProposalReceipt,
    due_date: ReminderDate,
    due_time: Option<ReminderTime>,
) -> Result<CreatedReminder, RemindersProposalBridgeError> {
    Ok(CreatedReminder {
        reminder_id: ReminderId::parse(&receipt.reminder_id)
            .map_err(RemindersProposalBridgeError::from)?,
        list_id: ListId::parse(&receipt.list_id).map_err(RemindersProposalBridgeError::from)?,
        list_name: MORROW_PROPOSED_LIST_NAME.to_owned(),
        due_date,
        due_time,
    })
}

impl From<RemindersError> for RemindersProposalBridgeError {
    fn from(error: RemindersError) -> Self {
        match error {
            RemindersError::InvalidInput { field, reason } => Self::InvalidInput { field, reason },
            RemindersError::PermissionDenied { operation } => Self::PermissionDenied {
                reason: format!("{operation:?}"),
            },
            RemindersError::ReminderMissing { id } => Self::SaveFailed {
                reason: format!("reminder not found: {id}"),
            },
            RemindersError::ListMissing { id } => Self::SaveFailed {
                reason: format!("list not found: {id}"),
            },
            RemindersError::ProposedListMissing { source_id } => Self::SaveFailed {
                reason: format!("Morrow Proposed list missing in source {source_id}"),
            },
            RemindersError::ApprovedMutationRejected { reminder_id } => Self::SaveFailed {
                reason: format!("approved reminder cannot be mutated: {reminder_id}"),
            },
        }
    }
}

impl From<EventKitReminderProposalError> for RemindersProposalBridgeError {
    fn from(error: EventKitReminderProposalError) -> Self {
        match error {
            EventKitReminderProposalError::InvalidInput { field, reason } => {
                Self::InvalidInput { field, reason }
            }
            EventKitReminderProposalError::PermissionDenied { reason } => {
                Self::PermissionDenied { reason }
            }
            EventKitReminderProposalError::SourceUnavailable { reason }
            | EventKitReminderProposalError::Unavailable { reason } => Self::Unavailable { reason },
            EventKitReminderProposalError::SaveFailed { reason } => Self::SaveFailed { reason },
            EventKitReminderProposalError::EmptyIdentifier { field } => Self::SaveFailed {
                reason: format!("EventKit returned an empty {field}"),
            },
            EventKitReminderProposalError::TruncatedIdentifier { field } => Self::SaveFailed {
                reason: format!("EventKit returned a truncated {field}"),
            },
        }
    }
}
