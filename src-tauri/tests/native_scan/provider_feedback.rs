use morrow_lib::native_bridge::FakeNativeBridge;
use morrow_storage::{DetectionRouteLabel, FeedbackLabelValue, Store, SystemOutcomeLabel};

use super::support::{
    assert_counts, batch, chat, query_sqlite, raw_chat, scan_request_with_feedback_text, temp_db,
};
use super::trace_support::{ProviderInvalidJsonStub, RecordingTraceRecorder};

#[test]
fn native_scan_provider_invalid_json_snapshot_redacts_response() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-provider-invalid-json-feedback.sqlite")?;
    let messages = batch(vec![raw_chat(
        "provider-invalid-json",
        "msg-provider-invalid-json",
        "Maybe meet tomorrow?",
        None,
    )?]);
    let bridge = FakeNativeBridge::with_morrow_store_path(db_path.clone()).with_messages(messages);
    let request = scan_request_with_feedback_text(
        &[chat("provider-invalid-json", 3, &["p1", "p2", "p3"])],
        &[],
        true,
        true,
        1,
        0,
    )?;
    let provider = ProviderInvalidJsonStub;
    let recorder = RecordingTraceRecorder::default();

    // When
    let result = bridge
        .scan_selected_chats_with_provider_and_trace(request, &db_path, &provider, &recorder)
        .map_err(|error| error.to_string())?;

    // Then
    assert_counts(&result, (0, 0, 1, 0, 0));
    let store = Store::open(&db_path).map_err(|error| error.to_string())?;
    let cases = store.eval_cases().map_err(|error| error.to_string())?;
    assert_eq!(cases.len(), 2, "{cases:#?}");
    assert!(cases.iter().any(|case| {
        case.snapshot.route.as_deref() == Some("provider_rejection")
            && case.snapshot.reason_code.as_deref() == Some("provider_invalid_json")
            && case.label.label_value
                == FeedbackLabelValue::DetectionRoute(DetectionRouteLabel::ProviderRejected)
    }));
    assert!(cases.iter().any(|case| {
        case.snapshot.reason_code.as_deref() == Some("provider_invalid_json")
            && case.label.label_value
                == FeedbackLabelValue::SystemOutcome(SystemOutcomeLabel::FailedValidation)
    }));
    assert_provider_response_redacted(&db_path)
}

fn assert_provider_response_redacted(db_path: &std::path::Path) -> Result<(), String> {
    let serialized = query_sqlite(
        db_path,
        "SELECT route || '|' || reason_code || '|' || excerpt || '|' || privacy_metadata_json
         FROM feature_snapshots;",
    )?;
    assert!(
        serialized.contains("provider_rejection|provider_invalid_json"),
        "{serialized}"
    );
    for forbidden in [
        "RAW_PROVIDER_RESPONSE_SECRET",
        "not valid json",
        "provider-invalid-json",
        "msg-provider-invalid-json",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "snapshot leaked provider/native text {forbidden}: {serialized}"
        );
    }
    Ok(())
}
