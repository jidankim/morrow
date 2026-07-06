mod reminders;

use morrow_calendar::ProposedEvent;
use morrow_reminders::{
    CreatedReminder, FakeReminders, ReminderAdapter, ReminderDraft, RemindersError, SourceId,
};
use morrow_storage::{
    CandidateId as StorageCandidateId, CandidateKind, ExternalObjectMapping, QueuedProposal,
};

use crate::native_bridge::eventkit_proposal::EventKitProposalBridge;

use super::normalized_time::ReminderDueComponents;
use super::{
    external_proposal_error, CalendarProposalReceipt, ReminderProposalReceipt,
    ScanSelectedChatsError,
};
#[cfg(test)]
pub(in crate::native_bridge) use reminders::ReminderProposalClient;
pub(in crate::native_bridge) use reminders::{
    RemindersProposalBridge, RemindersProposalBridgeError,
};

pub(in crate::native_bridge) struct LocalProposalAdapter;

pub trait ProposalReplayAdapter {
    fn create_calendar_proposal(
        &self,
        event: ProposedEvent,
    ) -> Result<CalendarProposalReceipt, ScanSelectedChatsError>;

    fn create_reminder_proposal(
        &self,
        candidate_id: &StorageCandidateId,
        selected_source_id: SourceId,
        due_components: ReminderDueComponents,
        reminder: ReminderDraft,
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
        _candidate_id: &StorageCandidateId,
        selected_source_id: SourceId,
        _due_components: ReminderDueComponents,
        reminder: ReminderDraft,
    ) -> Result<ReminderProposalReceipt, ScanSelectedChatsError> {
        let mut reminders = FakeReminders::allowed();
        let created = ReminderAdapter::new(selected_source_id)
            .create_proposal(&mut reminders, reminder)
            .map_err(reminders_bridge_error)?;
        Ok(ReminderProposalReceipt {
            reminder_id: created.reminder_id.to_string(),
            list_id: created.list_id.to_string(),
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
        candidate_id: &StorageCandidateId,
        selected_source_id: SourceId,
        due_components: ReminderDueComponents,
        reminder: ReminderDraft,
    ) -> Result<ReminderProposalReceipt, ScanSelectedChatsError> {
        RemindersProposalBridge::default()
            .create_proposal(candidate_id, selected_source_id, due_components, reminder)
            .map(reminder_receipt)
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

fn reminder_receipt(created: CreatedReminder) -> ReminderProposalReceipt {
    ReminderProposalReceipt {
        reminder_id: created.reminder_id.to_string(),
        list_id: created.list_id.to_string(),
    }
}

fn reminders_bridge_error(error: RemindersError) -> ScanSelectedChatsError {
    external_proposal_error(RemindersProposalBridgeError::from(error).to_string())
}
