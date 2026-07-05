use morrow_storage::{CandidateState, ExternalObjectMapping, ExternalSource, Store};

use super::super::fixtures::candidate_draft;
use super::super::*;
use super::fixtures::{
    assert_same_mapping, calendar_mapping, queued_calendar, CountingProposalAdapter,
};

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
