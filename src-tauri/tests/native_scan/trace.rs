use morrow_diagnostics::{TraceComponent, TraceDecision, TraceOperation, TraceOutcome};
use morrow_lib::native_bridge::FakeNativeBridge;

use super::support::{assert_counts, batch, chat, raw_chat, scan_request, temp_db};
use super::trace_support::{
    assert_trace_records_hide_raw_native_content, scan_result_counts, ProviderCandidateStub,
    ProviderUnavailableStub, RecordingTraceRecorder, UnavailableTraceRecorder,
};

#[test]
fn scan_selected_chats_records_provider_trace_with_fake_provider() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-provider-trace.sqlite")?;
    let messages = batch(vec![raw_chat(
        "provider-needed",
        "msg-provider",
        "Maybe meet tomorrow?",
        None,
    )?]);
    let bridge = FakeNativeBridge::with_morrow_store_path(db_path.clone()).with_messages(messages);
    let request = scan_request(
        &[chat("provider-needed", 3, &["p1", "p2", "p3"])],
        &[],
        false,
        1,
        0,
    )?;
    let provider = ProviderCandidateStub;
    let recorder = RecordingTraceRecorder::default();

    // When
    let result = bridge
        .scan_selected_chats_with_provider_and_trace(request, &db_path, &provider, &recorder)
        .map_err(|error| error.to_string())?;

    // Then
    assert_counts(&result, (1, 1, 0, 1, 0));
    let records = recorder.records()?;
    assert!(
        records
            .iter()
            .any(|record| record.span.component == TraceComponent::Provider
                && record.span.operation == TraceOperation::ProviderResult
                && record.span.reason_code.as_deref() == Some("provider_extract_success")),
        "{records:#?}"
    );
    assert!(
        records
            .iter()
            .any(|record| record.span.component == TraceComponent::Threshold
                && record.span.decision == Some(TraceDecision::ConfidenceAccepted)),
        "{records:#?}"
    );
    assert!(
        records
            .iter()
            .any(|record| record.span.outcome == TraceOutcome::CandidateCreated),
        "{records:#?}"
    );
    assert_trace_records_hide_raw_native_content(
        &records,
        &[
            "provider-needed",
            "msg-provider",
            "Provider private title",
            "Maybe meet tomorrow?",
        ],
    )?;
    Ok(())
}

#[test]
fn scan_selected_chats_records_provider_unavailable_trace_as_quiet() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-provider-unavailable-trace.sqlite")?;
    let messages = batch(vec![raw_chat(
        "provider-unavailable",
        "msg-provider-unavailable",
        "Maybe meet tomorrow?",
        None,
    )?]);
    let bridge = FakeNativeBridge::with_morrow_store_path(db_path.clone()).with_messages(messages);
    let request = scan_request(
        &[chat("provider-unavailable", 3, &["p1", "p2", "p3"])],
        &[],
        false,
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
    assert_counts(&result, (0, 0, 1, 0, 0));
    let records = recorder.records()?;
    assert!(
        records
            .iter()
            .any(|record| record.span.component == TraceComponent::Provider
                && record.span.decision == Some(TraceDecision::ProviderUnavailable)
                && record.span.outcome == TraceOutcome::QuietLogged),
        "{records:#?}"
    );
    assert!(
        records
            .iter()
            .any(|record| record.span.component == TraceComponent::Outcome
                && record.span.outcome == TraceOutcome::QuietLogged
                && record.span.reason_code.as_deref() == Some("provider_unavailable")),
        "{records:#?}"
    );
    assert!(
        !records
            .iter()
            .any(|record| record.span.outcome == TraceOutcome::CandidateCreated),
        "{records:#?}"
    );
    assert_trace_records_hide_raw_native_content(
        &records,
        &[
            "provider-unavailable",
            "msg-provider-unavailable",
            "Maybe meet tomorrow?",
        ],
    )?;
    Ok(())
}

#[test]
fn scan_selected_chats_unavailable_trace_recorder_leaves_counts_identical() -> Result<(), String> {
    // Given
    let (_recording_dir, recording_db_path) = temp_db("native-scan-recording-trace-counts.sqlite")?;
    let (_unavailable_dir, unavailable_db_path) =
        temp_db("native-scan-unavailable-trace-counts.sqlite")?;
    let recording_bridge = FakeNativeBridge::with_morrow_store_path(recording_db_path.clone())
        .with_messages(batch(vec![raw_chat(
            "provider-counts",
            "msg-provider-counts",
            "Maybe meet tomorrow?",
            None,
        )?]));
    let unavailable_bridge = FakeNativeBridge::with_morrow_store_path(unavailable_db_path.clone())
        .with_messages(batch(vec![raw_chat(
            "provider-counts",
            "msg-provider-counts",
            "Maybe meet tomorrow?",
            None,
        )?]));
    let provider = ProviderCandidateStub;
    let recording_request = scan_request(
        &[chat("provider-counts", 3, &["p1", "p2", "p3"])],
        &[],
        false,
        1,
        0,
    )?;
    let unavailable_request = scan_request(
        &[chat("provider-counts", 3, &["p1", "p2", "p3"])],
        &[],
        false,
        1,
        0,
    )?;
    let recording_recorder = RecordingTraceRecorder::default();
    let unavailable_recorder = UnavailableTraceRecorder;

    // When
    let recording_result = recording_bridge
        .scan_selected_chats_with_provider_and_trace(
            recording_request,
            &recording_db_path,
            &provider,
            &recording_recorder,
        )
        .map_err(|error| error.to_string())?;
    let unavailable_result = unavailable_bridge
        .scan_selected_chats_with_provider_and_trace(
            unavailable_request,
            &unavailable_db_path,
            &provider,
            &unavailable_recorder,
        )
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(
        scan_result_counts(&unavailable_result),
        scan_result_counts(&recording_result)
    );
    Ok(())
}
