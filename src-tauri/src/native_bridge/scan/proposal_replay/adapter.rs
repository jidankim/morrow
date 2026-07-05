use morrow_calendar::ProposedEvent;
use morrow_storage::{CandidateKind, ExternalObjectMapping, QueuedProposal};

use crate::native_bridge::eventkit_proposal::{EventKitProposalBridge, ReminderProposalRecord};

use super::{
    external_proposal_error, CalendarProposalReceipt, ReminderProposalReceipt,
    ScanSelectedChatsError,
};

pub(in crate::native_bridge) struct LocalProposalAdapter;

pub trait ProposalReplayAdapter {
    fn create_calendar_proposal(
        &self,
        event: ProposedEvent,
    ) -> Result<CalendarProposalReceipt, ScanSelectedChatsError>;

    fn create_reminder_proposal(
        &self,
        reminder: ReminderProposalRecord,
    ) -> Result<ReminderProposalReceipt, ScanSelectedChatsError>;

    fn create_legacy_proposal(
        &self,
        candidate: &QueuedProposal,
    ) -> Result<ExternalObjectMapping, ScanSelectedChatsError>;
}

impl ProposalReplayAdapter for LocalProposalAdapter {
    fn create_calendar_proposal(
        &self,
        event: ProposedEvent,
    ) -> Result<CalendarProposalReceipt, ScanSelectedChatsError> {
        Ok(CalendarProposalReceipt {
            event_id: format!(
                "morrow-local-calendar-{}",
                event.metadata.candidate_id.as_str()
            ),
            source_id: event.metadata.source_id.as_str().to_owned(),
        })
    }

    fn create_reminder_proposal(
        &self,
        reminder: ReminderProposalRecord,
    ) -> Result<ReminderProposalReceipt, ScanSelectedChatsError> {
        Ok(ReminderProposalReceipt {
            reminder_id: format!("morrow-local-reminder-{}", reminder.candidate_id.as_str()),
            list_id: "morrow-local-reminders".to_owned(),
        })
    }

    fn create_legacy_proposal(
        &self,
        candidate: &QueuedProposal,
    ) -> Result<ExternalObjectMapping, ScanSelectedChatsError> {
        match candidate.kind {
            CandidateKind::CalendarEvent => Err(external_proposal_error(
                "calendar candidates require calendar proposal payload replay",
            )),
            CandidateKind::TaskReminder => Err(external_proposal_error(
                "task reminder candidates require reminder proposal payload replay",
            )),
            CandidateKind::EventUpdate
            | CandidateKind::EventReschedule
            | CandidateKind::EventCancellation
            | CandidateKind::ReminderUpdate
            | CandidateKind::ReminderReschedule
            | CandidateKind::ReminderCancellation => Err(external_proposal_error(
                "proposal creation is unavailable for mutation candidates",
            )),
        }
    }
}

impl ProposalReplayAdapter for EventKitProposalBridge {
    fn create_calendar_proposal(
        &self,
        event: ProposedEvent,
    ) -> Result<CalendarProposalReceipt, ScanSelectedChatsError> {
        self.propose_event(event)
            .map(|receipt| CalendarProposalReceipt {
                event_id: receipt.event_id,
                source_id: receipt.source_id,
            })
            .map_err(|error| external_proposal_error(error.to_string()))
    }

    fn create_reminder_proposal(
        &self,
        reminder: ReminderProposalRecord,
    ) -> Result<ReminderProposalReceipt, ScanSelectedChatsError> {
        self.propose_reminder(reminder)
            .map(|receipt| ReminderProposalReceipt {
                reminder_id: receipt.reminder_id,
                list_id: receipt.list_id,
            })
            .map_err(|error| external_proposal_error(error.to_string()))
    }

    fn create_legacy_proposal(
        &self,
        _candidate: &QueuedProposal,
    ) -> Result<ExternalObjectMapping, ScanSelectedChatsError> {
        Err(external_proposal_error(
            "EventKit proposal adapter cannot create legacy proposal kinds",
        ))
    }
}
