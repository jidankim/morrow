use morrow_storage::ExternalSource;

use super::super::calendar::{calendar_mapping_from_payload, proposed_event_from_payload};
use super::super::fixtures::{calendar_payload, CANDIDATE_ID};
use super::*;

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
        ("2026-07-15T14:00:00[America/Los_Angeles]", 1_784_149_200),
        ("2026-01-15T14:00:00[America/Los_Angeles]", 1_768_514_400),
        ("2026-07-15T14:00:00[Europe/London]", 1_784_120_400),
        ("2026-01-15T14:00:00[Europe/London]", 1_768_485_600),
        ("2026-07-15T14:00:00[Asia/Tokyo]", 1_784_091_600),
        ("2026-07-15T14:00:00[UTC]", 1_784_124_000),
        ("2026-07-15T14:00:00Z", 1_784_124_000),
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
    let payload = calendar_payload("2026-07-15T14:00:00[Mars/Olympus_Mons]");

    // When
    let error = proposed_event_from_payload(&payload).expect_err("unsupported timezone rejects");

    // Then
    match error {
        ScanSelectedChatsError::ExternalProposal(message) => {
            assert!(message.contains("unsupported normalized_time timezone"));
            assert!(!message.contains("Mars/Olympus_Mons"));
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn calendar_payload_rejects_dst_boundary_timezone_without_guessing() {
    // Given
    let payload = calendar_payload("2026-03-08T02:30:00[America/Los_Angeles]");

    // When
    let error = proposed_event_from_payload(&payload).expect_err("dst gap rejects");

    // Then
    match error {
        ScanSelectedChatsError::ExternalProposal(message) => {
            assert!(message.contains("ambiguous or invalid normalized_time timezone"));
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
