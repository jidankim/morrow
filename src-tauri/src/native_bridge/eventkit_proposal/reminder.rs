use std::fmt::{Display, Formatter};

use morrow_calendar::{format_notes, CalendarSourceId, CandidateId, ProposalMetadata};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReminderProposalRecord {
    pub candidate_id: CandidateId,
    pub selected_source_id: CalendarSourceId,
    pub title: String,
    pub notes: String,
    pub due: ReminderDueComponents,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReminderDueComponents {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub time_zone: ReminderDueTimeZone,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReminderDueTimeZone {
    Named(String),
    Utc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventKitReminderProposalReceipt {
    pub reminder_id: String,
    pub list_id: String,
    pub source_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKitReminderProposalError {
    InvalidInput { field: &'static str, reason: String },
    PermissionDenied { reason: String },
    SourceUnavailable { reason: String },
    SaveFailed { reason: String },
    EmptyIdentifier { field: &'static str },
    TruncatedIdentifier { field: &'static str },
    Unavailable { reason: String },
}

impl Display for EventKitReminderProposalError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput { field, reason } => {
                write!(formatter, "invalid Reminders proposal {field}: {reason}")
            }
            Self::PermissionDenied { reason } => {
                write!(formatter, "Reminders permission denied: {reason}")
            }
            Self::SourceUnavailable { reason } => {
                write!(formatter, "Reminders source unavailable: {reason}")
            }
            Self::SaveFailed { reason } => {
                write!(formatter, "Reminders reminder save failed: {reason}")
            }
            Self::EmptyIdentifier { field } => write!(
                formatter,
                "Reminders reminder save failed: EventKit returned an empty {field}"
            ),
            Self::TruncatedIdentifier { field } => write!(
                formatter,
                "Reminders reminder save failed: EventKit returned a truncated {field}"
            ),
            Self::Unavailable { reason } => {
                write!(formatter, "Reminders proposal bridge unavailable: {reason}")
            }
        }
    }
}

impl std::error::Error for EventKitReminderProposalError {}

pub(super) fn format_reminder_record(
    reminder: ReminderProposalRecord,
) -> Result<ReminderProposalRecord, EventKitReminderProposalError> {
    let ReminderProposalRecord {
        candidate_id,
        selected_source_id,
        title,
        notes,
        due,
    } = reminder;
    reject_nul("title", &title)?;
    reject_nul("notes", &notes)?;
    reject_nul("selected_source_id", selected_source_id.as_str())?;

    let metadata = ProposalMetadata {
        candidate_id: candidate_id.clone(),
        source_id: selected_source_id.clone(),
    };
    Ok(ReminderProposalRecord {
        candidate_id,
        selected_source_id,
        title,
        notes: format_notes(&notes, None, &metadata),
        due,
    })
}

pub(super) fn validate_reminder_receipt(
    receipt: EventKitReminderProposalReceipt,
) -> Result<EventKitReminderProposalReceipt, EventKitReminderProposalError> {
    validate_identifier("reminder_id", &receipt.reminder_id)?;
    validate_identifier("list_id", &receipt.list_id)?;
    validate_identifier("source_id", &receipt.source_id)?;
    Ok(receipt)
}

fn reject_nul(field: &'static str, value: &str) -> Result<(), EventKitReminderProposalError> {
    if value.contains('\0') {
        return Err(EventKitReminderProposalError::InvalidInput {
            field,
            reason: "must not contain NUL bytes".to_owned(),
        });
    }
    Ok(())
}

fn validate_identifier(
    field: &'static str,
    value: &str,
) -> Result<(), EventKitReminderProposalError> {
    const MAX_EVENTKIT_IDENTIFIER_BYTES: usize = 255;
    if value.trim().is_empty() {
        return Err(EventKitReminderProposalError::EmptyIdentifier { field });
    }
    if value.len() > MAX_EVENTKIT_IDENTIFIER_BYTES {
        return Err(EventKitReminderProposalError::TruncatedIdentifier { field });
    }
    Ok(())
}
