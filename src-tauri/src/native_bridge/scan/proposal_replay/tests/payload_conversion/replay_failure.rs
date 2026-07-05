use morrow_storage::{CandidateState, QueuedProposal, Store};

use super::super::fixtures::candidate_draft;
use super::*;

#[test]
fn replay_external_proposals_records_adapter_failure_detail() {
    // Given
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("morrow.sqlite")).expect("store");
    let candidate_id = store
        .create_candidate(candidate_draft("msg-replay-failure"))
        .expect("candidate");
    store
        .transition_candidate(
            &candidate_id,
            CandidateState::CreatingExternal,
            "cap_selected_for_external_creation",
            1_782_352_399,
        )
        .expect("creating external");
    let queued = QueuedProposal::new(
        candidate_id.as_str(),
        "public-chat",
        900,
        "2026-07-15T14:00:00Z",
        false,
    )
    .expect("queued proposal");
    let adapter = FakeProposalAdapter::failing_calendar(
        "Calendar source unavailable: no writable calendar source. Retry failed after a long \
         sanitized diagnostic that must be bounded before it is persisted to candidate audit \
         history so replay failure evidence cannot overflow the storage reason contract.",
    );

    // When
    let summary = replay_external_proposals(&store, &[queued], &adapter).expect("replay");

    // Then
    assert_eq!(summary.created, 0);
    assert_eq!(summary.failed, 1);
    assert_eq!(
        store.candidate_state(&candidate_id).expect("state"),
        CandidateState::Failed
    );
    let audit = store.audit_entries(&candidate_id).expect("audit");
    let failure_reason = audit
        .iter()
        .find(|entry| {
            entry
                .reason
                .starts_with("external_proposal_creation_failed: Calendar source unavailable")
        })
        .expect("bounded failure audit")
        .reason
        .as_str();
    assert!(failure_reason.len() <= 240, "{failure_reason}");
}
