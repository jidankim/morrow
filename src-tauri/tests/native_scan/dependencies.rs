use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
};

use morrow_calendar::ProposedEvent;
use morrow_detection::{AiProvider, ProviderError, ProviderRequest, ProviderResponse};
use morrow_lib::native_bridge::eventkit_proposal::ReminderProposalRecord;
use morrow_lib::native_bridge::{
    CalendarProposalReceipt, ProposalReplayAdapter, ReminderProposalReceipt, ScanSelectedChatsError,
};
use morrow_storage::{ExternalObjectMapping, QueuedProposal};

#[derive(Debug, Clone, Copy)]
pub struct CandidateProvider;

impl AiProvider for CandidateProvider {
    fn extract(&self, _request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        Ok(ProviderResponse::new(
            "{\"kind\":\"calendar_event\",\"title\":\"Provider supplied title\",\
             \"confidence_millis\":800,\
             \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
             \"anchor_message_guid\":\"beta-provider-route\",\
             \"evidence_message_guids\":[\"beta-provider-route\"]}",
        ))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TaskReminderProvider;

impl AiProvider for TaskReminderProvider {
    fn extract(&self, _request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        Ok(ProviderResponse::new(
            "{\"kind\":\"task_reminder\",\"title\":\"Finish review of the essay\",\
             \"confidence_millis\":820,\
             \"normalized_time\":\"2026-07-25T09:00:00[Asia/Seoul]\",\
             \"anchor_message_guid\":\"beta-provider-route\",\
             \"evidence_message_guids\":[\"beta-provider-route\"]}",
        ))
    }
}

#[derive(Debug)]
pub struct CountingProvider<P> {
    inner: P,
    calls: Cell<usize>,
}

impl<P> CountingProvider<P> {
    pub const fn new(inner: P) -> Self {
        Self {
            inner,
            calls: Cell::new(0),
        }
    }

    pub fn calls(&self) -> usize {
        self.calls.get()
    }
}

impl<P: AiProvider> AiProvider for CountingProvider<P> {
    fn extract(&self, request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        self.calls.set(self.calls.get() + 1);
        self.inner.extract(request)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UnavailableTestProvider;

impl AiProvider for UnavailableTestProvider {
    fn extract(&self, _request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        Err(ProviderError::Unavailable {
            reason: "test provider unavailable".to_owned(),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InvalidJsonProvider;

impl AiProvider for InvalidJsonProvider {
    fn extract(&self, _request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        Ok(ProviderResponse::new("not json"))
    }
}

#[derive(Debug, Default)]
pub struct RecordingProposalAdapter {
    created_titles: RefCell<Vec<String>>,
    created_by_candidate: RefCell<BTreeMap<String, String>>,
    created_reminders_by_candidate: RefCell<BTreeMap<String, String>>,
    fail_calendar: bool,
    reminder_failure: Option<String>,
}

impl RecordingProposalAdapter {
    pub fn created_titles(&self) -> Vec<String> {
        self.created_titles.borrow().clone()
    }

    pub fn failing_calendar() -> Self {
        Self {
            fail_calendar: true,
            ..Self::default()
        }
    }

    pub fn failing_reminder_permission_denied() -> Self {
        Self {
            reminder_failure: Some(
                "Reminders permission denied: test reminders unavailable".to_owned(),
            ),
            ..Self::default()
        }
    }

    pub fn created_count(&self) -> usize {
        self.created_by_candidate.borrow().len()
    }

    pub fn created_reminder_count(&self) -> usize {
        self.created_reminders_by_candidate.borrow().len()
    }
}

impl ProposalReplayAdapter for RecordingProposalAdapter {
    fn create_calendar_proposal(
        &self,
        event: ProposedEvent,
    ) -> Result<CalendarProposalReceipt, ScanSelectedChatsError> {
        if self.fail_calendar {
            return Err(ScanSelectedChatsError::ExternalProposal(
                "Calendar permission denied: test calendar unavailable".to_owned(),
            ));
        }
        let candidate_id = event.metadata.candidate_id.as_str().to_owned();
        let event_id = {
            let mut created = self.created_by_candidate.borrow_mut();
            let next = created.len() + 1;
            created
                .entry(candidate_id)
                .or_insert_with(|| format!("event-injected-{next}"))
                .clone()
        };
        self.created_titles.borrow_mut().push(event.title);
        Ok(CalendarProposalReceipt {
            event_id,
            source_id: "source-injected-1".to_owned(),
        })
    }

    fn create_reminder_proposal(
        &self,
        reminder: ReminderProposalRecord,
    ) -> Result<ReminderProposalReceipt, ScanSelectedChatsError> {
        if let Some(reason) = &self.reminder_failure {
            return Err(ScanSelectedChatsError::ExternalProposal(reason.clone()));
        }
        let candidate_id = reminder.candidate_id.as_str().to_owned();
        let reminder_id = {
            let mut created = self.created_reminders_by_candidate.borrow_mut();
            let next = created.len() + 1;
            created
                .entry(candidate_id)
                .or_insert_with(|| format!("reminder-injected-{next}"))
                .clone()
        };
        self.created_titles.borrow_mut().push(reminder.title);
        Ok(ReminderProposalReceipt {
            reminder_id,
            list_id: "reminders-list-injected-1".to_owned(),
        })
    }

    fn create_legacy_proposal(
        &self,
        _candidate: &QueuedProposal,
    ) -> Result<ExternalObjectMapping, ScanSelectedChatsError> {
        Err(ScanSelectedChatsError::ExternalProposal(
            "legacy proposal replay is outside this test".to_owned(),
        ))
    }
}
