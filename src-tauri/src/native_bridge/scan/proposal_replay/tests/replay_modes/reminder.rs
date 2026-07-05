use morrow_storage::{CandidateKind, CandidateState, ExternalObjectMapping, ExternalSource, Store};

use super::super::*;
use super::fixtures::{
    assert_same_mapping, queued_reminder, queued_reminder_update, reminder_draft, reminder_mapping,
    reminder_update_draft, CountingProposalAdapter,
};

#[test]
fn task_reminder_replay_creates_reminders_mapping_without_legacy_branch() {
    // Given
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("morrow.sqlite")).expect("store");
    let candidate_id = store
        .create_candidate(reminder_draft("msg-reminder-replay"))
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
    let mapping = reminder_mapping(&store, &candidate_id);
    assert_eq!(mapping.external_object_id, "fake-reminder-1");
    assert_eq!(mapping.external_source_id, "fake-reminder-list-1");
}

#[test]
fn task_reminder_legacy_adapter_rejects_without_creating_mapping() {
    // Given
    let queued = QueuedProposal::with_kind(
        "morrow_0000000000000001",
        CandidateKind::TaskReminder,
        "public-chat",
        900,
        "2026-07-25T09:00:00[Asia/Seoul]",
        false,
    )
    .expect("queued reminder");
    let adapter = LocalProposalAdapter;

    // When
    let error = adapter
        .create_legacy_proposal(&queued)
        .expect_err("task reminders reject legacy creation");

    // Then
    match error {
        ScanSelectedChatsError::ExternalProposal(message) => {
            assert!(message.contains("reminder proposal payload replay"));
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn partial_reminder_receipt_replay_uses_reminders_mapping_without_adapter_call() {
    // Given
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("morrow.sqlite")).expect("store");
    let candidate_id = store
        .create_candidate(reminder_draft("msg-reminder-partial-write"))
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
        source: ExternalSource::Reminders,
        external_object_id: "fake-reminder-partial".to_owned(),
        external_source_id: "fake-reminder-list-1".to_owned(),
        mapped_at: MAPPED_AT,
    };
    store
        .record_candidate_external_receipt(&partial_mapping)
        .expect("partial reminder receipt");
    let queued = queued_reminder(&candidate_id);
    let adapter = CountingProposalAdapter::default();

    // When
    let summary = replay_external_proposals(&store, &[queued], &adapter).expect("replay");
    let recovered = reminder_mapping(&store, &candidate_id);

    // Then
    assert_eq!(summary.created, 0);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.calendar_commit_idempotency, 0);
    assert_eq!(adapter.calendar_calls(), 0);
    assert_eq!(adapter.reminder_calls(), 0);
    assert_eq!(adapter.legacy_calls(), 0);
    assert_same_mapping(&partial_mapping, &recovered);
    assert_eq!(
        store.candidate_state(&candidate_id).expect("state"),
        CandidateState::Visible
    );
}

#[test]
fn reminder_update_replay_uses_unsupported_legacy_branch_without_reminders_create() {
    // Given
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("morrow.sqlite")).expect("store");
    let candidate_id = store
        .create_candidate(reminder_update_draft("msg-reminder-update"))
        .expect("candidate");
    store
        .transition_candidate(
            &candidate_id,
            CandidateState::CreatingExternal,
            "cap_selected_for_external_creation",
            1_782_352_399,
        )
        .expect("creating external");
    let queued = queued_reminder_update(&candidate_id);
    let adapter = CountingProposalAdapter::default();

    // When
    let summary = replay_external_proposals(&store, &[queued], &adapter).expect("replay");

    // Then
    assert_eq!(summary.created, 0);
    assert_eq!(summary.failed, 1);
    assert_eq!(adapter.calendar_calls(), 0);
    assert_eq!(adapter.reminder_calls(), 0);
    assert_eq!(adapter.legacy_calls(), 1);
    assert_eq!(
        store.candidate_state(&candidate_id).expect("state"),
        CandidateState::Failed
    );
    let audit = store.audit_entries(&candidate_id).expect("audit");
    let failure_reason = audit
        .iter()
        .find(|entry| {
            entry.reason.starts_with(
                "external_proposal_creation_failed: legacy proposal unsupported by fake",
            )
        })
        .expect("bounded mutation failure audit")
        .reason
        .as_str();
    assert!(failure_reason.len() <= 240, "{failure_reason}");
    assert!(
        !failure_reason.contains("source hidden"),
        "{failure_reason}"
    );
}
