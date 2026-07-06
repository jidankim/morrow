use std::cell::{Cell, RefCell};

use morrow_calendar::ProposedEvent;
use morrow_reminders::{ReminderDraft, SourceId};
use morrow_storage::{
    CandidateDraft, CandidateId as StorageCandidateId, CandidateKind, ExternalObjectMapping,
    ExternalSource, QueuedProposal, Store,
};

use super::super::*;

#[derive(Default)]
pub(super) struct CountingProposalAdapter {
    calendar_calls: Cell<usize>,
    reminder_calls: Cell<usize>,
    legacy_calls: Cell<usize>,
    calendar_events: RefCell<Vec<ProposedEvent>>,
    reminders: RefCell<Vec<ReminderDraft>>,
}

impl CountingProposalAdapter {
    pub(super) fn calendar_calls(&self) -> usize {
        self.calendar_calls.get()
    }

    pub(super) fn legacy_calls(&self) -> usize {
        self.legacy_calls.get()
    }

    pub(super) fn reminder_calls(&self) -> usize {
        self.reminder_calls.get()
    }

    pub(super) fn reminder_drafts(&self) -> Vec<ReminderDraft> {
        self.reminders.borrow().clone()
    }
}

impl ProposalReplayAdapter for CountingProposalAdapter {
    fn create_calendar_proposal(
        &self,
        event: ProposedEvent,
    ) -> Result<CalendarProposalReceipt, ScanSelectedChatsError> {
        self.calendar_calls.set(self.calendar_calls.get() + 1);
        self.calendar_events.borrow_mut().push(event);
        Ok(CalendarProposalReceipt {
            event_id: format!("fake-event-{}", self.calendar_calls.get()),
            source_id: "fake-source-1".to_owned(),
        })
    }

    fn create_reminder_proposal(
        &self,
        _candidate_id: &StorageCandidateId,
        _selected_source_id: SourceId,
        _due_components: ReminderDueComponents,
        reminder: ReminderDraft,
    ) -> Result<ReminderProposalReceipt, ScanSelectedChatsError> {
        self.reminder_calls.set(self.reminder_calls.get() + 1);
        self.reminders.borrow_mut().push(reminder);
        Ok(ReminderProposalReceipt {
            reminder_id: format!("fake-reminder-{}", self.reminder_calls.get()),
            list_id: "fake-reminder-list-1".to_owned(),
        })
    }

    fn create_legacy_proposal(
        &self,
        _candidate: &QueuedProposal,
    ) -> Result<ExternalObjectMapping, ScanSelectedChatsError> {
        self.legacy_calls.set(self.legacy_calls.get() + 1);
        Err(ScanSelectedChatsError::ExternalProposal(
            "legacy proposal unsupported by fake".to_owned(),
        ))
    }
}

pub(super) fn queued_calendar(candidate_id: &StorageCandidateId) -> QueuedProposal {
    QueuedProposal::new(
        candidate_id.as_str(),
        "public-chat",
        900,
        "2026-07-15T14:00:00Z",
        false,
    )
    .expect("queued proposal")
}

pub(super) fn queued_reminder(candidate_id: &StorageCandidateId) -> QueuedProposal {
    QueuedProposal::with_kind(
        candidate_id.as_str(),
        CandidateKind::TaskReminder,
        "public-chat",
        900,
        "2026-07-25T09:00:00[Asia/Seoul]",
        false,
    )
    .expect("queued reminder")
}

pub(super) fn queued_reminder_update(candidate_id: &StorageCandidateId) -> QueuedProposal {
    QueuedProposal::with_kind(
        candidate_id.as_str(),
        CandidateKind::ReminderUpdate,
        "public-chat",
        900,
        "2026-07-25T09:00:00[Asia/Seoul]",
        false,
    )
    .expect("queued reminder update")
}

pub(super) fn calendar_mapping(
    store: &Store,
    candidate_id: &StorageCandidateId,
) -> ExternalObjectMapping {
    store
        .candidate_external_mapping(candidate_id, ExternalSource::Calendar, MAPPED_AT)
        .expect("mapping")
        .expect("calendar mapping")
}

pub(super) fn reminder_mapping(
    store: &Store,
    candidate_id: &StorageCandidateId,
) -> ExternalObjectMapping {
    store
        .candidate_external_mapping(candidate_id, ExternalSource::Reminders, MAPPED_AT)
        .expect("mapping")
        .expect("reminder mapping")
}

pub(super) fn reminder_draft(anchor_message_guid: &str) -> CandidateDraft {
    CandidateDraft {
        kind: CandidateKind::TaskReminder,
        chat_guid: "public-chat".to_owned(),
        anchor_message_guid: anchor_message_guid.to_owned(),
        title: "Daily list: 2 anchovies; 3 salmon".to_owned(),
        confidence_millis: 900,
        normalized_time: "2026-07-07T23:59:00[Asia/Seoul]".to_owned(),
        evidence_excerpt: "source hidden".to_owned(),
        observed_at: 1_782_352_398,
    }
}

pub(super) fn reminder_update_draft(anchor_message_guid: &str) -> CandidateDraft {
    CandidateDraft {
        kind: CandidateKind::ReminderUpdate,
        chat_guid: "public-chat".to_owned(),
        anchor_message_guid: anchor_message_guid.to_owned(),
        title: "Finish review of the essay".to_owned(),
        confidence_millis: 900,
        normalized_time: "2026-07-25T09:00:00[Asia/Seoul]".to_owned(),
        evidence_excerpt: "source hidden".to_owned(),
        observed_at: 1_782_352_398,
    }
}

pub(super) fn assert_same_mapping(left: &ExternalObjectMapping, right: &ExternalObjectMapping) {
    assert_eq!(left.candidate_id.as_str(), right.candidate_id.as_str());
    assert_eq!(left.source, right.source);
    assert_eq!(left.external_object_id, right.external_object_id);
    assert_eq!(left.external_source_id, right.external_source_id);
    assert_eq!(left.mapped_at, right.mapped_at);
}
