use morrow_calendar::{CalendarSourceId, CandidateId};

use super::{
    EventKitProposalClient, EventKitProposalError, EventKitProposalReceipt,
    EventKitReminderProposalError, EventKitReminderProposalReceipt, ReminderDueComponents,
    ReminderDueTimeZone, ReminderProposalRecord,
};

#[derive(Debug)]
pub(super) struct FakeEventKitClient {
    pub(super) reminder_receipt:
        Result<EventKitReminderProposalReceipt, EventKitReminderProposalError>,
    pub(super) created_reminder: Option<ReminderProposalRecord>,
}

impl FakeEventKitClient {
    pub(super) fn succeeding() -> Self {
        Self {
            reminder_receipt: Ok(EventKitReminderProposalReceipt {
                reminder_id: "reminder-1".to_owned(),
                list_id: "list-morrow-proposed".to_owned(),
                source_id: "source-reminders-local".to_owned(),
            }),
            created_reminder: None,
        }
    }

    pub(super) fn failing_reminder(error: EventKitReminderProposalError) -> Self {
        Self {
            reminder_receipt: Err(error),
            created_reminder: None,
        }
    }
}

impl EventKitProposalClient for FakeEventKitClient {
    fn create_proposal_event(
        &mut self,
        _record: &morrow_calendar::EventRecord,
    ) -> Result<EventKitProposalReceipt, EventKitProposalError> {
        Err(EventKitProposalError::Unavailable {
            reason: "calendar path is not used by reminder tests".to_owned(),
        })
    }

    fn create_proposal_reminder(
        &mut self,
        record: &ReminderProposalRecord,
    ) -> Result<EventKitReminderProposalReceipt, EventKitReminderProposalError> {
        self.created_reminder = Some(record.clone());
        self.reminder_receipt.clone()
    }
}

pub(super) fn reminder_record() -> ReminderProposalRecord {
    ReminderProposalRecord {
        candidate_id: CandidateId::new("candidate-reminder-1").expect("candidate id is valid"),
        selected_source_id: CalendarSourceId::new("morrow-selected-reminders")
            .expect("source id is valid"),
        title: "Messages reminder candidate".to_owned(),
        notes: "Handle after lunch".to_owned(),
        due: ReminderDueComponents {
            year: 2026,
            month: 7,
            day: 15,
            hour: 14,
            minute: 30,
            second: 0,
            time_zone: ReminderDueTimeZone::Named("Asia/Seoul".to_owned()),
        },
    }
}
