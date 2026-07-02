use std::fs;

use morrow_diagnostics::diagnostics_trace_dir;
use morrow_lib::native_bridge::{
    load_decision_evidence_at, CodexAuthStatus, DecisionEvidenceTraceRetention,
    LoadDecisionEvidenceRequest,
};

use super::support::{
    assert_counts, auth_readiness, candidate_json, scan_request_at, FakeCodexOutcome,
    RecordingCodexRunner, RejectingProposalAdapter, ScanFixture,
};

#[test]
fn decision_evidence_reports_not_retained_when_local_diagnostics_disabled() -> Result<(), String> {
    // Given
    let fixture = ScanFixture::new("codex-decision-evidence-disabled")?;
    let request = scan_request_at(1_782_352_400)?;
    let runner = RecordingCodexRunner::new(vec![FakeCodexOutcome::WriteOutput(
        candidate_json().to_owned(),
    )]);
    let adapter = RejectingProposalAdapter;
    let scan = fixture.scan_with_request_and_app_data_dir(
        request,
        auth_readiness(CodexAuthStatus::LoggedInUsingChatGpt, true),
        &runner,
        &adapter,
    )?;
    assert_counts(&scan, (1, 1, 0, 0, 1));

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
    let item = provider_candidate_item(&report)?;
    assert_eq!(
        item.trace_retention,
        DecisionEvidenceTraceRetention::NotRetained
    );
    assert!(item.trace_sequence.is_empty());
    Ok(())
}

#[test]
fn decision_evidence_reports_diagnostics_missing_after_deleted_trace_files() -> Result<(), String> {
    // Given
    let fixture = ScanFixture::new("codex-decision-evidence-deleted")?;
    let mut request = scan_request_at(1_782_352_400)?;
    request.local_diagnostics_enabled = true;
    request.local_diagnostics_retention_days = 30;
    let runner = RecordingCodexRunner::new(vec![FakeCodexOutcome::WriteOutput(
        candidate_json().to_owned(),
    )]);
    let adapter = RejectingProposalAdapter;
    let scan = fixture.scan_with_request_and_app_data_dir(
        request,
        auth_readiness(CodexAuthStatus::LoggedInUsingChatGpt, true),
        &runner,
        &adapter,
    )?;
    assert_counts(&scan, (1, 1, 0, 0, 1));
    fs::remove_dir_all(diagnostics_trace_dir(fixture.app_data_dir()))
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
    let item = provider_candidate_item(&report)?;
    assert_eq!(
        item.trace_retention,
        DecisionEvidenceTraceRetention::DiagnosticsMissing
    );
    assert!(item.trace_sequence.is_empty());
    Ok(())
}

#[test]
fn decision_evidence_reports_diagnostics_missing_when_trace_read_fails() -> Result<(), String> {
    // Given
    let fixture = ScanFixture::new("codex-decision-evidence-trace-read-error")?;
    let mut request = scan_request_at(1_782_352_400)?;
    request.local_diagnostics_enabled = true;
    request.local_diagnostics_retention_days = 30;
    let runner = RecordingCodexRunner::new(vec![FakeCodexOutcome::WriteOutput(
        candidate_json().to_owned(),
    )]);
    let adapter = RejectingProposalAdapter;
    let scan = fixture.scan_with_request_and_app_data_dir(
        request,
        auth_readiness(CodexAuthStatus::LoggedInUsingChatGpt, true),
        &runner,
        &adapter,
    )?;
    assert_counts(&scan, (1, 1, 0, 0, 1));
    let traces_dir = diagnostics_trace_dir(fixture.app_data_dir());
    fs::remove_dir_all(&traces_dir).map_err(|error| error.to_string())?;
    fs::create_dir_all(traces_dir.join("trace-1782352400-0.jsonl"))
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
    assert_eq!(report.skipped_trace_line_count, 0);
    let item = provider_candidate_item(&report)?;
    assert_eq!(
        item.trace_retention,
        DecisionEvidenceTraceRetention::DiagnosticsMissing
    );
    assert!(item.trace_sequence.is_empty());
    Ok(())
}

fn provider_candidate_item(
    report: &morrow_lib::native_bridge::DecisionEvidenceReport,
) -> Result<&morrow_lib::native_bridge::DecisionEvidenceItem, String> {
    report
        .items
        .iter()
        .find(|item| item.route.as_deref() == Some("provider_candidate"))
        .ok_or_else(|| format!("provider candidate evidence item missing: {report:#?}"))
}
