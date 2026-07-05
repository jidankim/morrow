use std::cell::{Cell, RefCell};

use morrow_calendar::ProposedEvent;
use morrow_storage::{CandidateState, ExternalObjectMapping};

use crate::native_bridge::eventkit_proposal::ProposedReminder;

use super::*;

#[test]
fn dry_run_calendar_replay_computes_action_without_adapter_call() {
    // Given
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("morrow.sqlite")).expect("store");
    let candidate_id = store
        .create_candidate(candidate_draft("msg-dry-run"))
        .expect("candidate");
    store
        .transition_candidate(
            &candidate_id,
            CandidateState::CreatingExternal,
            "cap_selected_for_external_creation",
            1_782_352_399,
        )
        .expect("creating external");
    let queued = queued_calendar(&candidate_id);
    let adapter = CountingProposalAdapter::default();

    // When
    let summary =
        replay_external_proposals_dry_run(&store, &[queued], &adapter).expect("dry-run replay");

    // Then
    assert_eq!(summary.created, 0);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.dry_run, 1);
    assert_eq!(adapter.calendar_calls(), 0);
    assert_eq!(adapter.legacy_calls(), 0);
    assert_eq!(
        store.candidate_state(&candidate_id).expect("state"),
        CandidateState::CreatingExternal
    );
    assert!(store
        .candidate_external_mapping(&candidate_id, ExternalSource::Calendar, MAPPED_AT)
        .expect("mapping")
        .is_none());
}

#[test]
fn repeat_calendar_commit_reuses_mapping_and_records_idempotency() {
    // Given
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("morrow.sqlite")).expect("store");
    let candidate_id = store
        .create_candidate(candidate_draft("msg-repeat-commit"))
        .expect("candidate");
    store
        .transition_candidate(
            &candidate_id,
            CandidateState::CreatingExternal,
            "cap_selected_for_external_creation",
            1_782_352_399,
        )
        .expect("creating external");
    let queued = queued_calendar(&candidate_id);
    let adapter = CountingProposalAdapter::default();

    // When
    let first = replay_external_proposals(&store, std::slice::from_ref(&queued), &adapter)
        .expect("first replay");
    let first_mapping = calendar_mapping(&store, &candidate_id);
    let second = replay_external_proposals(&store, &[queued], &adapter).expect("second replay");
    let second_mapping = calendar_mapping(&store, &candidate_id);

    // Then
    assert_eq!(first.created, 1);
    assert_eq!(first.failed, 0);
    assert_eq!(first.calendar_commit_idempotency, 0);
    assert_eq!(second.created, 0);
    assert_eq!(second.failed, 0);
    assert_eq!(second.calendar_commit_idempotency, 1);
    assert_eq!(adapter.calendar_calls(), 1);
    assert_eq!(adapter.legacy_calls(), 0);
    assert_same_mapping(&first_mapping, &second_mapping);
    assert_eq!(first_mapping.external_object_id, "fake-event-1");
    assert_eq!(first_mapping.external_source_id, "fake-source-1");
    let audit = store.audit_entries(&candidate_id).expect("audit");
    assert!(audit
        .iter()
        .any(|entry| entry.reason == "calendar_commit_idempotency"));
}

#[test]
fn partial_write_replay_uses_existing_mapping_without_duplicate_external_create() {
    // Given
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("morrow.sqlite")).expect("store");
    let candidate_id = store
        .create_candidate(candidate_draft("msg-partial-write"))
        .expect("candidate");
    store
        .transition_candidate(
            &candidate_id,
            CandidateState::CreatingExternal,
            "cap_selected_for_external_creation",
            1_782_352_399,
        )
        .expect("creating external");
    let partial_mapping = ExternalObjectMapping {
        candidate_id: candidate_id.clone(),
        source: ExternalSource::Calendar,
        external_object_id: "fake-event-partial".to_owned(),
        external_source_id: "fake-source-1".to_owned(),
        mapped_at: MAPPED_AT,
    };
    store
        .record_candidate_external_receipt(&partial_mapping)
        .expect("partial receipt");
    let queued = queued_calendar(&candidate_id);
    let adapter = CountingProposalAdapter::default();

    // When
    let summary = replay_external_proposals(&store, &[queued], &adapter).expect("replay");
    let recovered = calendar_mapping(&store, &candidate_id);

    // Then
    assert_eq!(summary.created, 0);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.calendar_commit_idempotency, 0);
    assert_eq!(adapter.calendar_calls(), 0);
    assert_eq!(adapter.legacy_calls(), 0);
    assert_same_mapping(&partial_mapping, &recovered);
    assert_eq!(
        store.candidate_state(&candidate_id).expect("state"),
        CandidateState::Visible
    );
}

#[test]
fn task_reminder_replay_creates_reminders_mapping_without_legacy_branch() {
    // Given
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("morrow.sqlite")).expect("store");
    let candidate_id = store
        .create_candidate(task_reminder_draft("msg-reminder-replay"))
        .expect("candidate");
    store
        .transition_candidate(
            &candidate_id,
            CandidateState::CreatingExternal,
            "cap_selected_for_external_creation",
            1_782_352_399,
        )
        .expect("creating external");
    let queued = queued_reminder(&candidate_id);
    let adapter = CountingProposalAdapter::default();

    // When
    let summary = replay_external_proposals(&store, &[queued], &adapter).expect("replay");

    // Then
    assert_eq!(summary.created, 1);
    assert_eq!(summary.failed, 0);
    assert_eq!(adapter.reminder_calls(), 1);
    assert_eq!(adapter.legacy_calls(), 0);
    assert_eq!(
        store.candidate_state(&candidate_id).expect("state"),
        CandidateState::Visible
    );
    let mapping = store
        .candidate_external_mapping(&candidate_id, ExternalSource::Reminders, MAPPED_AT)
        .expect("mapping")
        .expect("reminder mapping");
    assert_eq!(mapping.external_object_id, "fake-reminder-1");
    assert_eq!(mapping.external_source_id, "fake-reminders-source-1");
}

#[derive(Default)]
struct CountingProposalAdapter {
    calendar_calls: Cell<usize>,
    reminder_calls: Cell<usize>,
    legacy_calls: Cell<usize>,
    calendar_events: RefCell<Vec<ProposedEvent>>,
    reminders: RefCell<Vec<ProposedReminder>>,
}

impl CountingProposalAdapter {
    fn calendar_calls(&self) -> usize {
        self.calendar_calls.get()
    }

    fn legacy_calls(&self) -> usize {
        self.legacy_calls.get()
    }

    fn reminder_calls(&self) -> usize {
        self.reminder_calls.get()
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
        reminder: ProposedReminder,
    ) -> Result<ReminderProposalReceipt, ScanSelectedChatsError> {
        self.reminder_calls.set(self.reminder_calls.get() + 1);
        self.reminders.borrow_mut().push(reminder);
        Ok(ReminderProposalReceipt {
            reminder_id: format!("fake-reminder-{}", self.reminder_calls.get()),
            source_id: "fake-reminders-source-1".to_owned(),
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

fn queued_calendar(candidate_id: &StorageCandidateId) -> QueuedProposal {
    QueuedProposal::new(
        candidate_id.as_str(),
        "public-chat",
        900,
        "2026-07-15T14:00:00Z",
        false,
    )
    .expect("queued proposal")
}

fn queued_reminder(candidate_id: &StorageCandidateId) -> QueuedProposal {
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

fn task_reminder_draft(anchor_message_guid: &str) -> CandidateDraft {
    CandidateDraft {
        kind: CandidateKind::TaskReminder,
        chat_guid: "public-chat".to_owned(),
        anchor_message_guid: anchor_message_guid.to_owned(),
        title: "Finish review of the essay".to_owned(),
        confidence_millis: 900,
        normalized_time: "2026-07-25T09:00:00[Asia/Seoul]".to_owned(),
        evidence_excerpt: "source hidden".to_owned(),
        observed_at: 1_782_352_398,
    }
}

fn calendar_mapping(store: &Store, candidate_id: &StorageCandidateId) -> ExternalObjectMapping {
    store
        .candidate_external_mapping(candidate_id, ExternalSource::Calendar, MAPPED_AT)
        .expect("mapping")
        .expect("calendar mapping")
}

fn assert_same_mapping(left: &ExternalObjectMapping, right: &ExternalObjectMapping) {
    assert_eq!(left.candidate_id.as_str(), right.candidate_id.as_str());
    assert_eq!(left.source, right.source);
    assert_eq!(left.external_object_id, right.external_object_id);
    assert_eq!(left.external_source_id, right.external_source_id);
    assert_eq!(left.mapped_at, right.mapped_at);
}
