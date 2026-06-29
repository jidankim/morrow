use std::cell::RefCell;

use morrow_calendar::ProposedEvent;
use morrow_storage::{
    CalendarProposalPayload, CandidateDraft, CandidateId as StorageCandidateId, CandidateState,
    ExternalSource, Store,
};

use super::calendar::{
    calendar_mapping_from_payload, proposed_event_from_payload, CalendarProposalReceipt,
};
use super::*;

const CANDIDATE_ID: &str = "morrow_0000000000000001";

#[test]
fn calendar_payload_converts_to_proposed_event_with_safe_defaults() {
    // Given
    let payload = calendar_payload("2026-07-15T14:00:00Z");

    // When
    let event = proposed_event_from_payload(&payload).expect("calendar payload converts");

    // Then
    assert_eq!(event.title, "Messages event candidate");
    assert_eq!(event.time_range.start_unix, 1_784_124_000);
    assert_eq!(event.time_range.end_unix, 1_784_125_800);
    assert_eq!(event.user_note, "");
    assert_eq!(event.video_url, None);
    assert_eq!(event.metadata.candidate_id.as_str(), CANDIDATE_ID);
    assert_eq!(
        event.metadata.source_id.as_str(),
        "morrow-selected-messages"
    );
}

#[test]
fn calendar_payload_converts_app_timezone_normalized_time() {
    // Given
    let payload = calendar_payload("2026-07-15T14:00:00[Asia/Seoul]");

    // When
    let event = proposed_event_from_payload(&payload).expect("timezone payload converts");

    // Then
    assert_eq!(event.time_range.start_unix, 1_784_091_600);
    assert_eq!(event.time_range.end_unix, 1_784_093_400);
}

#[test]
fn calendar_payload_converts_all_selectable_app_timezones() {
    // Given
    let cases = [
        ("2026-07-15T14:00:00[America/New_York]", 1_784_138_400),
        ("2026-01-15T14:00:00[America/New_York]", 1_768_503_600),
        ("2026-07-15T14:00:00[Europe/London]", 1_784_120_400),
        ("2026-01-15T14:00:00[Europe/London]", 1_768_485_600),
        ("2026-07-15T14:00:00[UTC]", 1_784_124_000),
    ];

    for (normalized_time, expected_start) in cases {
        // When
        let event = proposed_event_from_payload(&calendar_payload(normalized_time))
            .expect("selectable timezone converts");

        // Then
        assert_eq!(
            event.time_range.start_unix, expected_start,
            "{normalized_time}"
        );
    }
}

#[test]
fn calendar_payload_rejects_malformed_normalized_time() {
    // Given
    let payload = calendar_payload("tomorrow at 2");

    // When
    let error = proposed_event_from_payload(&payload).expect_err("malformed time rejects");

    // Then
    assert_external_error_contains(error, "normalized_time");
}

#[test]
fn calendar_payload_rejects_unsupported_app_timezone_without_echoing_value() {
    // Given
    let payload = calendar_payload("2026-07-15T14:00:00[America/Los_Angeles]");

    // When
    let error = proposed_event_from_payload(&payload).expect_err("unsupported timezone rejects");

    // Then
    match error {
        ScanSelectedChatsError::ExternalProposal(message) => {
            assert!(message.contains("unsupported normalized_time timezone"));
            assert!(!message.contains("America/Los_Angeles"));
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn calendar_payload_mapping_uses_adapter_receipt() {
    // Given
    let payload = calendar_payload("2026-07-15T14:00:00Z");
    let adapter = FakeProposalAdapter::default();

    // When
    let mapping =
        calendar_mapping_from_payload(&payload, &adapter).expect("calendar mapping created");

    // Then
    assert_eq!(mapping.candidate_id.as_str(), CANDIDATE_ID);
    assert_eq!(mapping.source, ExternalSource::Calendar);
    assert_eq!(mapping.external_object_id, "fake-event-1");
    assert_eq!(mapping.external_source_id, "fake-source-1");
    assert_eq!(mapping.mapped_at, 1_782_352_400);

    let captured = adapter.captured_event.borrow();
    let event = captured.as_ref().expect("event captured");
    assert_eq!(event.time_range.start_unix, 1_784_124_000);
}

#[test]
fn task_reminder_local_mapping_stays_legacy() {
    // Given
    let candidate = QueuedProposal::with_kind(
        CANDIDATE_ID,
        CandidateKind::TaskReminder,
        "public-chat",
        800,
        "2026-07-15T14:00:00Z",
        false,
    )
    .expect("queued reminder");

    // When
    let mapping = LocalProposalAdapter
        .create_legacy_proposal(&candidate)
        .expect("legacy reminder mapping");

    // Then
    assert_eq!(mapping.source, ExternalSource::Reminders);
    assert_eq!(
        mapping.external_object_id,
        "morrow-local-reminders-morrow_0000000000000001"
    );
    assert_eq!(mapping.external_source_id, "morrow-local-reminders");
}

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
        "Calendar source unavailable: no writable calendar source",
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
    assert!(audit.iter().any(|entry| entry.reason
        == "external_proposal_creation_failed: Calendar source unavailable: no writable calendar source"));
}

#[derive(Default)]
struct FakeProposalAdapter {
    captured_event: RefCell<Option<ProposedEvent>>,
    calendar_error: Option<&'static str>,
}

impl FakeProposalAdapter {
    const fn failing_calendar(message: &'static str) -> Self {
        Self {
            captured_event: RefCell::new(None),
            calendar_error: Some(message),
        }
    }
}

impl ProposalReplayAdapter for FakeProposalAdapter {
    fn create_calendar_proposal(
        &self,
        event: ProposedEvent,
    ) -> Result<CalendarProposalReceipt, ScanSelectedChatsError> {
        self.captured_event.replace(Some(event));
        if let Some(message) = self.calendar_error {
            return Err(ScanSelectedChatsError::ExternalProposal(message.to_owned()));
        }
        Ok(CalendarProposalReceipt {
            event_id: "fake-event-1".to_owned(),
            source_id: "fake-source-1".to_owned(),
        })
    }

    fn create_legacy_proposal(
        &self,
        _candidate: &QueuedProposal,
    ) -> Result<ExternalObjectMapping, ScanSelectedChatsError> {
        Err(ScanSelectedChatsError::ExternalProposal(
            "legacy proposal unsupported by fake".to_owned(),
        ))
    }
}

fn calendar_payload(normalized_time: &str) -> CalendarProposalPayload {
    CalendarProposalPayload {
        candidate_id: StorageCandidateId::from_storage(CANDIDATE_ID).expect("candidate id"),
        kind: CandidateKind::CalendarEvent,
        normalized_time: normalized_time.to_owned(),
        title: "Messages event candidate".to_owned(),
        source_id: "morrow-selected-messages".to_owned(),
    }
}

fn candidate_draft(anchor_message_guid: &str) -> CandidateDraft {
    CandidateDraft {
        kind: CandidateKind::CalendarEvent,
        chat_guid: "public-chat".to_owned(),
        anchor_message_guid: anchor_message_guid.to_owned(),
        title: "Morrow QA".to_owned(),
        confidence_millis: 900,
        normalized_time: "2026-07-15T14:00:00Z".to_owned(),
        evidence_excerpt: "source hidden".to_owned(),
        observed_at: 1_782_352_398,
    }
}

fn assert_external_error_contains(error: ScanSelectedChatsError, expected: &str) {
    match error {
        ScanSelectedChatsError::ExternalProposal(message) => {
            assert!(message.contains(expected), "{message}");
        }
        other => panic!("unexpected error: {other}"),
    }
}
