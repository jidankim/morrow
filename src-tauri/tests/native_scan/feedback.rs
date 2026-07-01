use morrow_lib::native_bridge::FakeNativeBridge;
use morrow_storage::{DetectionRouteLabel, FeedbackLabelValue, Store};

use super::support::{
    assert_counts, batch, chat, query_sqlite, raw_chat, scan_request,
    scan_request_with_feedback_text, temp_db,
};
use super::trace_support::{
    ProviderLowConfidenceStub, ProviderUnavailableStub, RecordingTraceRecorder,
};

#[test]
fn native_scan_records_candidate_feature_snapshot_and_visible_label() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-candidate-feedback.sqlite")?;
    let messages = batch(vec![raw_chat(
        "candidate-feedback",
        "msg-candidate-feedback",
        "Let's meet 2026-07-15 14:00 at the private clinic.",
        None,
    )?]);
    let bridge = FakeNativeBridge::with_morrow_store_path(db_path.clone()).with_messages(messages);
    let request = scan_request_with_feedback_text(
        &[chat("candidate-feedback", 3, &["p1", "p2", "p3"])],
        &[],
        true,
        true,
        1,
        0,
    )?;
    let provider = ProviderUnavailableStub;
    let recorder = RecordingTraceRecorder::default();

    // When
    let result = bridge
        .scan_selected_chats_with_provider_and_trace(request, &db_path, &provider, &recorder)
        .map_err(|error| error.to_string())?;

    // Then
    assert_counts(&result, (1, 1, 0, 1, 0));
    let store = Store::open(&db_path).map_err(|error| error.to_string())?;
    let cases = store.eval_cases().map_err(|error| error.to_string())?;
    assert_eq!(cases.len(), 1, "{cases:#?}");
    let case = &cases[0];
    assert_eq!(
        case.snapshot.route.as_deref(),
        Some("deterministic_candidate")
    );
    assert_eq!(case.snapshot.confidence_millis, Some(850));
    assert_eq!(case.snapshot.participant_count, Some(3));
    assert_eq!(case.snapshot.tapback_signal.as_deref(), Some("absent"));
    assert_eq!(
        case.snapshot.excerpt.as_deref(),
        Some("Let's meet 2026-07-15 14:00 at the private clinic.")
    );
    assert!(case.snapshot.meta.diagnostics.trace_id.is_some());
    assert!(case.snapshot.meta.diagnostics.span_id.is_some());
    assert!(case.snapshot.meta.diagnostics.chat_hash.is_some());
    assert!(case.snapshot.meta.diagnostics.message_hash.is_some());
    assert_eq!(
        case.label.label_value,
        FeedbackLabelValue::DetectionRoute(DetectionRouteLabel::DeterministicCandidate)
    );
    let events = query_sqlite(
        &db_path,
        "SELECT event_type FROM feedback_events ORDER BY id;",
    )?;
    assert_eq!(events.trim(), "candidate_visible");
    Ok(())
}

#[test]
fn native_scan_records_quiet_snapshot_and_feedback_labels() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-quiet-feedback.sqlite")?;
    let provider = ProviderLowConfidenceStub;
    let unavailable_provider = ProviderUnavailableStub;
    let recorder = RecordingTraceRecorder::default();

    // When
    scan_quiet_case(QuietCase {
        db_path: &db_path,
        chat_guid: "quiet-deterministic",
        message_guid: "msg-quiet-deterministic",
        text: "Status update only.",
        provider: &provider,
        recorder: &recorder,
    })?;
    scan_quiet_case(QuietCase {
        db_path: &db_path,
        chat_guid: "quiet-threshold",
        message_guid: "msg-quiet-threshold",
        text: "Maybe meet tomorrow?",
        provider: &provider,
        recorder: &recorder,
    })?;
    scan_quiet_case(QuietCase {
        db_path: &db_path,
        chat_guid: "quiet-unavailable",
        message_guid: "msg-quiet-unavailable",
        text: "Maybe schedule the planning hold tomorrow?",
        provider: &unavailable_provider,
        recorder: &recorder,
    })?;

    // Then
    let store = Store::open(&db_path).map_err(|error| error.to_string())?;
    let cases = store.eval_cases().map_err(|error| error.to_string())?;
    assert_eq!(cases.len(), 5, "{cases:#?}");
    let rows = query_sqlite(
        &db_path,
        "SELECT subject_id || '|' || label_type || '|' || label_value FROM labels ORDER BY label_key;",
    )?;
    assert!(
        rows.contains("deterministic_stop:no_scheduling_signal"),
        "{rows}"
    );
    assert!(rows.contains("|detection_route|quiet_stop"), "{rows}");
    assert!(rows.contains("confidence_below_threshold"), "{rows}");
    assert!(
        rows.contains("|detection_route|provider_rejected"),
        "{rows}"
    );
    assert!(rows.contains("|proposal_outcome|unknown"), "{rows}");
    assert!(rows.contains("provider_unavailable"), "{rows}");
    assert!(
        rows.contains("|detection_route|provider_unavailable"),
        "{rows}"
    );
    assert!(rows.contains("|system_outcome|failed_provider"), "{rows}");
    assert_quiet_snapshot_rows(&db_path)
}

#[test]
fn native_scan_feature_feedback_is_idempotent_for_repeated_scans() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-feedback-idempotent.sqlite")?;
    let messages = batch(vec![raw_chat(
        "idempotent-feedback",
        "msg-idempotent-feedback",
        "Let's meet 2026-07-15 14:00 at the private clinic.",
        None,
    )?]);
    let bridge = FakeNativeBridge::with_morrow_store_path(db_path.clone()).with_messages(messages);
    let provider = ProviderUnavailableStub;
    let recorder = RecordingTraceRecorder::default();

    // When
    for _ in 0..2 {
        bridge
            .scan_selected_chats_with_provider_and_trace(
                scan_request(
                    &[chat("idempotent-feedback", 3, &["p1", "p2", "p3"])],
                    &[],
                    false,
                    1,
                    0,
                )?,
                &db_path,
                &provider,
                &recorder,
            )
            .map_err(|error| error.to_string())?;
    }

    // Then
    let counts = query_sqlite(
        &db_path,
        "SELECT
            (SELECT COUNT(*) FROM feature_snapshots) || '|' ||
            (SELECT COUNT(*) FROM feedback_events) || '|' ||
            (SELECT COUNT(*) FROM labels);",
    )?;
    assert_eq!(counts.trim(), "1|1|1");
    Ok(())
}

struct QuietCase<'a, P> {
    db_path: &'a std::path::Path,
    chat_guid: &'a str,
    message_guid: &'a str,
    text: &'a str,
    provider: &'a P,
    recorder: &'a RecordingTraceRecorder,
}

fn scan_quiet_case<P: morrow_detection::AiProvider>(input: QuietCase<'_, P>) -> Result<(), String> {
    let bridge = FakeNativeBridge::with_morrow_store_path(input.db_path.to_path_buf())
        .with_messages(batch(vec![raw_chat(
            input.chat_guid,
            input.message_guid,
            input.text,
            None,
        )?]));
    bridge
        .scan_selected_chats_with_provider_and_trace(
            scan_request(
                &[chat(input.chat_guid, 3, &["p1", "p2", "p3"])],
                &[],
                false,
                1,
                0,
            )?,
            input.db_path,
            input.provider,
            input.recorder,
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn assert_quiet_snapshot_rows(db_path: &std::path::Path) -> Result<(), String> {
    let snapshots = query_sqlite(
        db_path,
        "SELECT route || '|' || reason_code || '|' || excerpt FROM feature_snapshots ORDER BY snapshot_key;",
    )?;
    assert!(
        snapshots.contains("deterministic_stop|deterministic_stop:no_scheduling_signal|Source excerpt hidden by settings."),
        "{snapshots}"
    );
    assert!(
        snapshots.contains(
            "provider_rejection|confidence_below_threshold|Source excerpt hidden by settings."
        ),
        "{snapshots}"
    );
    assert!(
        snapshots
            .contains("provider_rejection|provider_unavailable|Source excerpt hidden by settings."),
        "{snapshots}"
    );
    Ok(())
}
