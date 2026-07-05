use std::cell::RefCell;

use morrow_calendar::ProposedEvent;
use morrow_storage::{ExternalObjectMapping, QueuedProposal};

use crate::native_bridge::eventkit_proposal::ReminderProposalRecord;

use super::calendar::CalendarProposalReceipt;
use super::*;

mod calendar;
mod reminder;
mod replay_failure;

#[derive(Default)]
struct FakeProposalAdapter {
    captured_event: RefCell<Option<ProposedEvent>>,
    calendar_error: Option<&'static str>,
}

impl FakeProposalAdapter {
    const fn failing_calendar(message: &'static str) -> Self {
        Self {
            captured_event: RefCell::new(None),
            calendar_error: Some(message),
        }
    }
}

impl ProposalReplayAdapter for FakeProposalAdapter {
    fn create_calendar_proposal(
        &self,
        event: ProposedEvent,
    ) -> Result<CalendarProposalReceipt, ScanSelectedChatsError> {
        self.captured_event.replace(Some(event));
        if let Some(message) = self.calendar_error {
            return Err(ScanSelectedChatsError::ExternalProposal(message.to_owned()));
        }
        Ok(CalendarProposalReceipt {
            event_id: "fake-event-1".to_owned(),
            source_id: "fake-source-1".to_owned(),
        })
    }

    fn create_reminder_proposal(
        &self,
        reminder: ReminderProposalRecord,
    ) -> Result<ReminderProposalReceipt, ScanSelectedChatsError> {
        Ok(ReminderProposalReceipt {
            reminder_id: format!("fake-reminder-{}", reminder.candidate_id.as_str()),
            list_id: "fake-list-1".to_owned(),
        })
    }

    fn create_legacy_proposal(
        &self,
        _candidate: &QueuedProposal,
    ) -> Result<ExternalObjectMapping, ScanSelectedChatsError> {
        Err(ScanSelectedChatsError::ExternalProposal(
            "legacy proposal unsupported by fake".to_owned(),
        ))
    }
}

fn assert_external_error_contains(error: ScanSelectedChatsError, expected: &str) {
    match error {
        ScanSelectedChatsError::ExternalProposal(message) => {
            assert!(message.contains(expected), "{message}");
        }
        other => panic!("unexpected error: {other}"),
    }
}
