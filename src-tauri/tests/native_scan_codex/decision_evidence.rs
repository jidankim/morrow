use std::fs;

use morrow_diagnostics::diagnostics_trace_dir;
use morrow_lib::native_bridge::{
    load_decision_evidence_at, CodexAuthStatus, DecisionEvidenceTraceRetention,
    LoadDecisionEvidenceRequest,
};

use super::support::{
    assert_counts, auth_readiness, candidate_json, scan_request_at, FakeCodexOutcome,
    RecordingCodexRunner, RejectingProposalAdapter, ScanFixture, FIXTURE_MESSAGE_TEXT,
    NATIVE_CHAT_ID, NATIVE_MESSAGE_ID, PRIVACY_CANARY,
};

#[test]
fn decision_evidence_attaches_retained_trace_sequence_when_diagnostics_jsonl_exists(
) -> Result<(), String> {
    // Given
    let fixture = ScanFixture::new("codex-decision-evidence-retained")?;
    let mut request = scan_request_at(1_782_352_400)?;
    request.local_diagnostics_enabled = true;
    request.local_diagnostics_retention_days = 30;
    let provider_output = candidate_json();
    let runner =
        RecordingCodexRunner::new(vec![FakeCodexOutcome::WriteOutput(provider_output.clone())]);
    let adapter = RejectingProposalAdapter;
    let scan = fixture.scan_with_request_and_app_data_dir(
        request,
        auth_readiness(CodexAuthStatus::LoggedInUsingChatGpt, true),
        &runner,
        &adapter,
    )?;
    assert_counts(&scan, (1, 1, 0, 0, 1));

    fs::write(
        diagnostics_trace_dir(fixture.app_data_dir()).join("trace-1782352400-99.jsonl"),
        "MORROW_PRIVACY_CANARY_RAW_TEXT corrupt unrelated line\n",
    )
    .map_err(|error| error.to_string())?;

    // When
    let report = load_decision_evidence_at(
        fixture.store_path(),
        fixture.app_data_dir(),
        LoadDecisionEvidenceRequest {
            limit: 10,
            created_candidate_ids: Vec::new(),
        },
    )?;

    // Then
    assert_eq!(report.skipped_trace_line_count, 1);
    let item = report
        .items
        .iter()
        .find(|item| item.route.as_deref() == Some("provider_candidate"))
        .ok_or_else(|| format!("provider candidate evidence item missing: {report:#?}"))?;
    assert_eq!(
        item.trace_retention,
        DecisionEvidenceTraceRetention::Retained
    );
    if item.trace_sequence.is_empty() {
        return Err(format!(
            "retained evidence item has no trace sequence: {item:#?}"
        ));
    }
    assert!(item
        .trace_sequence
        .iter()
        .any(|step| step.operation == "provider_result"));
    assert!(item
        .trace_sequence
        .iter()
        .any(|step| step.outcome == "candidate_created"));

    let serialized = serde_json::to_string(&report).map_err(|error| error.to_string())?;
    if let Ok(copy_path) = std::env::var("MORROW_TASK2_DECISION_EVIDENCE_COPY") {
        fs::write(copy_path, &serialized).map_err(|error| error.to_string())?;
    }
    for allowed_camel_case in [
        "\"skippedTraceLineCount\"",
        "\"traceRetention\"",
        "\"traceSequence\"",
        "\"latestEvalStatus\"",
    ] {
        if !serialized.contains(allowed_camel_case) {
            return Err(format!(
                "serialized command response omitted camelCase field {allowed_camel_case}: {serialized}"
            ));
        }
    }
    for forbidden in [
        FIXTURE_MESSAGE_TEXT,
        "Provider meeting",
        NATIVE_CHAT_ID,
        NATIVE_MESSAGE_ID,
        provider_output.as_str(),
        "prompt",
        "response",
        "embedding",
        PRIVACY_CANARY,
        fixture.app_data_dir().to_string_lossy().as_ref(),
    ] {
        if serialized.contains(forbidden) {
            return Err(format!(
                "decision evidence response leaked forbidden fixture string: {forbidden}"
            ));
        }
    }
    Ok(())
}

#[test]
fn decision_evidence_sanitizes_unreadable_store_errors() -> Result<(), String> {
    // Given
    let fixture = ScanFixture::new("codex-decision-evidence-bad-store")?;
    let bad_store_path = fixture.app_data_dir().join("bad-store.sqlite");
    fs::create_dir_all(&bad_store_path).map_err(|error| error.to_string())?;

    // When
    let error = load_decision_evidence_at(
        &bad_store_path,
        fixture.app_data_dir(),
        LoadDecisionEvidenceRequest {
            limit: 10,
            created_candidate_ids: Vec::new(),
        },
    )
    .err()
    .ok_or_else(|| "decision evidence unexpectedly loaded from bad store path".to_owned())?;

    // Then
    if error != "decision evidence store unavailable" {
        return Err("decision evidence store error was not sanitized".to_owned());
    }
    if let Ok(copy_path) = std::env::var("MORROW_TASK2_DECISION_EVIDENCE_ERROR_COPY") {
        fs::write(copy_path, &error).map_err(|error| error.to_string())?;
    }
    for forbidden in [
        bad_store_path.to_string_lossy().as_ref(),
        fixture.app_data_dir().to_string_lossy().as_ref(),
    ] {
        if error.contains(forbidden) {
            return Err("decision evidence error leaked a forbidden path".to_owned());
        }
    }
    Ok(())
}
