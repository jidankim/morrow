use std::cell::Cell;

use morrow_calendar::ProposedEvent;
use morrow_reminders::{ReminderDraft, SourceId};
use morrow_storage::{
    CandidateDraft, CandidateKind, CandidateState, ExternalObjectMapping, QueuedProposal, Store,
};

use super::super::{replay_external_proposals, CalendarProposalReceipt, ProposalReplayAdapter};
use super::ScanSelectedChatsError;

#[test]
fn replay_external_proposals_marks_invalid_normalized_time_without_echoing_message_text() {
    // Given
    let invalid_time = "2026-07-15T14:00:00[Africa/Nowhere]";
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("morrow.sqlite")).expect("store");
    let candidate_id = store
        .create_candidate(candidate_draft("msg-invalid-time", invalid_time))
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
        invalid_time,
        false,
    )
    .expect("queued proposal");
    let adapter = RejectingAdapter::default();

    // When
    let summary = replay_external_proposals(&store, &[queued], &adapter).expect("replay");

    // Then
    assert_eq!(summary.created, 0);
    assert_eq!(summary.failed, 1);
    assert_eq!(
        store.candidate_state(&candidate_id).expect("state"),
        CandidateState::Failed
    );
    assert!(
        !adapter.calendar_called.get(),
        "invalid time should fail before adapter call"
    );
    let audit = store.audit_entries(&candidate_id).expect("audit");
    let failure_reason = audit
        .iter()
        .find(|entry| {
            entry.reason.starts_with(
                "external_proposal_creation_failed: unsupported normalized_time timezone",
            )
        })
        .expect("invalid time failure audit")
        .reason
        .as_str();
    assert!(failure_reason.len() <= 240, "{failure_reason}");
    assert!(
        !failure_reason.contains("Africa/Nowhere"),
        "{failure_reason}"
    );
    assert!(
        !failure_reason.contains("source hidden"),
        "{failure_reason}"
    );
}

#[derive(Default)]
struct RejectingAdapter {
    calendar_called: Cell<bool>,
}

impl ProposalReplayAdapter for RejectingAdapter {
    fn create_calendar_proposal(
        &self,
        _event: ProposedEvent,
    ) -> Result<CalendarProposalReceipt, ScanSelectedChatsError> {
        self.calendar_called.set(true);
        Err(ScanSelectedChatsError::ExternalProposal(
            "calendar adapter should not be called".to_owned(),
        ))
    }

    fn create_reminder_proposal(
        &self,
        _candidate_id: &morrow_storage::CandidateId,
        _selected_source_id: SourceId,
        _due_components: super::ReminderDueComponents,
        _reminder: ReminderDraft,
    ) -> Result<super::super::ReminderProposalReceipt, ScanSelectedChatsError> {
        Err(ScanSelectedChatsError::ExternalProposal(
            "reminder proposal unsupported by normalized_time tests".to_owned(),
        ))
    }

    fn create_legacy_proposal(
        &self,
        _candidate: &QueuedProposal,
    ) -> Result<ExternalObjectMapping, ScanSelectedChatsError> {
        Err(ScanSelectedChatsError::ExternalProposal(
            "legacy proposal unsupported by normalized_time tests".to_owned(),
        ))
    }
}

fn candidate_draft(anchor_message_guid: &str, normalized_time: &str) -> CandidateDraft {
    CandidateDraft {
        kind: CandidateKind::CalendarEvent,
        chat_guid: "public-chat".to_owned(),
        anchor_message_guid: anchor_message_guid.to_owned(),
        title: "Morrow QA".to_owned(),
        confidence_millis: 900,
        normalized_time: normalized_time.to_owned(),
        evidence_excerpt: "source hidden".to_owned(),
        observed_at: 1_782_352_398,
    }
}
