use std::fmt::{Display, Formatter};

use morrow_calendar::{
    format_notes, Availability, EventRecord, ProposedEvent, PROPOSED_CALENDAR_NAME,
};

#[cfg(target_os = "macos")]
mod macos;
mod reminder;
#[cfg(test)]
mod reminder_test_support;
#[cfg(test)]
mod reminder_tests;
#[cfg(test)]
mod tests;

pub use reminder::{
    EventKitReminderProposalError, EventKitReminderProposalReceipt, ReminderDueComponents,
    ReminderDueTimeZone, ReminderProposalRecord,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventKitProposalReceipt {
    pub event_id: String,
    pub calendar_id: String,
    pub source_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKitProposalError {
    InvalidInput { field: &'static str, reason: String },
    PermissionDenied { reason: String },
    SourceUnavailable { reason: String },
    SaveFailed { reason: String },
    EmptyEventIdentifier,
    Unavailable { reason: String },
}

impl Display for EventKitProposalError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput { field, reason } => {
                write!(formatter, "invalid Calendar proposal {field}: {reason}")
            }
            Self::PermissionDenied { reason } => {
                write!(formatter, "Calendar permission denied: {reason}")
            }
            Self::SourceUnavailable { reason } => {
                write!(formatter, "Calendar source unavailable: {reason}")
            }
            Self::SaveFailed { reason } => {
                write!(formatter, "Calendar event save failed: {reason}")
            }
            Self::EmptyEventIdentifier => write!(
                formatter,
                "Calendar event save failed: EventKit returned an empty event identifier"
            ),
            Self::Unavailable { reason } => {
                write!(formatter, "Calendar proposal bridge unavailable: {reason}")
            }
        }
    }
}

impl std::error::Error for EventKitProposalError {}

#[derive(Debug, Default, Clone, Copy)]
pub struct EventKitProposalBridge;

impl EventKitProposalBridge {
    #[cfg(target_os = "macos")]
    pub fn propose_event(
        &self,
        event: ProposedEvent,
    ) -> Result<EventKitProposalReceipt, EventKitProposalError> {
        EventKitProposalAdapter::new(RealEventKitClient).propose_event(event)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn propose_event(
        &self,
        _event: ProposedEvent,
    ) -> Result<EventKitProposalReceipt, EventKitProposalError> {
        Err(EventKitProposalError::Unavailable {
            reason: "Calendar proposal creation requires macOS EventKit".to_owned(),
        })
    }

    #[cfg(target_os = "macos")]
    pub fn propose_reminder(
        &self,
        reminder: ReminderProposalRecord,
    ) -> Result<EventKitReminderProposalReceipt, EventKitReminderProposalError> {
        EventKitProposalAdapter::new(RealEventKitClient).propose_reminder(reminder)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn propose_reminder(
        &self,
        _reminder: ReminderProposalRecord,
    ) -> Result<EventKitReminderProposalReceipt, EventKitReminderProposalError> {
        Err(EventKitReminderProposalError::Unavailable {
            reason: "Reminders proposal creation requires macOS EventKit".to_owned(),
        })
    }
}

#[derive(Debug)]
struct EventKitProposalAdapter<C> {
    client: C,
}

impl<C: EventKitProposalClient> EventKitProposalAdapter<C> {
    const fn new(client: C) -> Self {
        Self { client }
    }

    fn propose_event(
        &mut self,
        event: ProposedEvent,
    ) -> Result<EventKitProposalReceipt, EventKitProposalError> {
        let ProposedEvent {
            title,
            time_range,
            user_note,
            video_url,
            metadata,
        } = event;
        let notes = format_notes(&user_note, video_url.as_ref(), &metadata);
        let record = EventRecord {
            calendar_name: PROPOSED_CALENDAR_NAME.to_owned(),
            calendar_source_id: metadata.source_id,
            title,
            time_range,
            availability: Availability::Free,
            alerts: Vec::new(),
            attendees: Vec::new(),
            invite_sent: false,
            notes,
        };
        let receipt = self.client.create_proposal_event(&record)?;
        if receipt.event_id.trim().is_empty() {
            return Err(EventKitProposalError::EmptyEventIdentifier);
        }
        Ok(receipt)
    }

    fn propose_reminder(
        &mut self,
        reminder: ReminderProposalRecord,
    ) -> Result<EventKitReminderProposalReceipt, EventKitReminderProposalError> {
        let record = reminder::format_reminder_record(reminder)?;
        let receipt = self.client.create_proposal_reminder(&record)?;
        reminder::validate_reminder_receipt(receipt)
    }
}

trait EventKitProposalClient {
    fn create_proposal_event(
        &mut self,
        record: &EventRecord,
    ) -> Result<EventKitProposalReceipt, EventKitProposalError>;

    fn create_proposal_reminder(
        &mut self,
        record: &ReminderProposalRecord,
    ) -> Result<EventKitReminderProposalReceipt, EventKitReminderProposalError>;
}

#[cfg(target_os = "macos")]
#[derive(Debug, Default, Clone, Copy)]
struct RealEventKitClient;

#[cfg(target_os = "macos")]
impl EventKitProposalClient for RealEventKitClient {
    fn create_proposal_event(
        &mut self,
        record: &EventRecord,
    ) -> Result<EventKitProposalReceipt, EventKitProposalError> {
        macos::create_proposal_event(record)
    }

    fn create_proposal_reminder(
        &mut self,
        record: &ReminderProposalRecord,
    ) -> Result<EventKitReminderProposalReceipt, EventKitReminderProposalError> {
        macos::create_proposal_reminder(record)
    }
}
