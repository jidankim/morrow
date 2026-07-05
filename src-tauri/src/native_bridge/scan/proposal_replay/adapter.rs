use morrow_calendar::ProposedEvent;
use morrow_storage::{CandidateKind, ExternalObjectMapping, ExternalSource, QueuedProposal};

use crate::native_bridge::eventkit_proposal::{EventKitProposalBridge, ProposedReminder};

use super::{
    external_proposal_error, CalendarProposalReceipt, ReminderProposalReceipt,
    ScanSelectedChatsError, MAPPED_AT,
};

pub(in crate::native_bridge) struct LocalProposalAdapter;

pub trait ProposalReplayAdapter {
    fn create_calendar_proposal(
        &self,
        event: ProposedEvent,
    ) -> Result<CalendarProposalReceipt, ScanSelectedChatsError>;

    fn create_reminder_proposal(
        &self,
        reminder: ProposedReminder,
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
        reminder: ProposedReminder,
    ) -> Result<ReminderProposalReceipt, ScanSelectedChatsError> {
        Ok(ReminderProposalReceipt {
            reminder_id: format!("morrow-local-reminder-{}", reminder.metadata.candidate_id),
            source_id: reminder.metadata.source_id,
        })
    }

    fn create_legacy_proposal(
        &self,
        candidate: &QueuedProposal,
    ) -> Result<ExternalObjectMapping, ScanSelectedChatsError> {
        let source = match candidate.kind {
            CandidateKind::CalendarEvent => {
                return Err(external_proposal_error(
                    "calendar candidates require calendar proposal payload replay",
                ));
            }
            CandidateKind::TaskReminder => ExternalSource::Reminders,
            CandidateKind::EventUpdate
            | CandidateKind::EventReschedule
            | CandidateKind::EventCancellation
            | CandidateKind::ReminderUpdate
            | CandidateKind::ReminderReschedule
            | CandidateKind::ReminderCancellation => {
                return Err(external_proposal_error(
                    "proposal creation is unavailable for mutation candidates",
                ));
            }
        };
        Ok(ExternalObjectMapping {
            candidate_id: candidate.candidate_id.clone(),
            source,
            external_object_id: format!(
                "morrow-local-{}-{}",
                source.as_str(),
                candidate.candidate_id.as_str()
            ),
            external_source_id: format!("morrow-local-{}", source.as_str()),
            mapped_at: MAPPED_AT,
        })
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
        reminder: ProposedReminder,
    ) -> Result<ReminderProposalReceipt, ScanSelectedChatsError> {
        self.propose_reminder(reminder)
            .map(|receipt| ReminderProposalReceipt {
                reminder_id: receipt.reminder_id,
                source_id: receipt.source_id,
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
