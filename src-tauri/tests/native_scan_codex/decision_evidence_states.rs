use std::fs;

use morrow_diagnostics::diagnostics_trace_dir;
use morrow_lib::native_bridge::{
    load_decision_evidence_at, CodexAuthStatus, DecisionEvidenceItem,
    DecisionEvidenceTraceRetention, LoadDecisionEvidenceRequest, NativeDecisionEvidenceSubjectType,
};

use super::support::{
    assert_counts, auth_readiness, candidate_json, scan_request_at, FakeCodexOutcome,
    RecordingCodexRunner, RejectingProposalAdapter, ScanFixture,
};

#[path = "decision_evidence_provider_rejection.rs"]
mod decision_evidence_provider_rejection;

#[test]
fn decision_evidence_reports_deleted_diagnostics_root_as_missing() -> Result<(), String> {
    // Given
    let fixture = ScanFixture::new("codex-decision-evidence-deleted-root")?;
    let mut request = scan_request_at(1_782_352_400)?;
    request.local_diagnostics_enabled = true;
    request.local_diagnostics_retention_days = 30;
    let runner = RecordingCodexRunner::new(vec![FakeCodexOutcome::WriteOutput(candidate_json())]);
    let adapter = RejectingProposalAdapter;
    let scan = fixture.scan_with_request_and_app_data_dir(
        request,
        auth_readiness(CodexAuthStatus::LoggedInUsingChatGpt, true),
        &runner,
        &adapter,
    )?;
    assert_counts(&scan, (1, 1, 0, 0, 1));
    fs::remove_dir_all(fixture.app_data_dir().join("diagnostics"))
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
    copy_report_if_requested("deleted-diagnostics-root.json", &report)?;

    // Then
    assert_eq!(report.items.len(), 1, "{report:#?}");
    let item = only_item(&report.items)?;
    assert_eq!(
        item.subject_type,
        NativeDecisionEvidenceSubjectType::Candidate
    );
    assert_eq!(item.route.as_deref(), Some("provider_candidate"));
    assert!(item.candidate_id.is_some());
    assert!(item.has_diagnostics_hashes);
    assert_eq!(
        item.trace_retention,
        DecisionEvidenceTraceRetention::DiagnosticsMissing
    );
    assert!(item.trace_sequence.is_empty());
    Ok(())
}

#[test]
fn decision_evidence_reports_sink_conflict_as_nonfatal_missing_trace() -> Result<(), String> {
    // Given
    let fixture = ScanFixture::new("codex-decision-evidence-sink-conflict")?;
    fs::create_dir_all(fixture.app_data_dir()).map_err(|error| error.to_string())?;
    fs::write(
        fixture.app_data_dir().join("diagnostics"),
        "regular file blocks diagnostics/traces creation",
    )
    .map_err(|error| error.to_string())?;
    let mut request = scan_request_at(1_782_352_400)?;
    request.local_diagnostics_enabled = true;
    request.local_diagnostics_retention_days = 30;
    let runner = RecordingCodexRunner::new(vec![FakeCodexOutcome::WriteOutput(candidate_json())]);
    let adapter = RejectingProposalAdapter;

    // When
    let scan = fixture.scan_with_request_and_app_data_dir(
        request,
        auth_readiness(CodexAuthStatus::LoggedInUsingChatGpt, true),
        &runner,
        &adapter,
    )?;
    let report = load_decision_evidence_at(
        fixture.store_path(),
        fixture.app_data_dir(),
        LoadDecisionEvidenceRequest {
            limit: 10,
            created_candidate_ids: Vec::new(),
        },
    )?;
    copy_report_if_requested("sink-conflict.json", &report)?;

    // Then
    assert_counts(&scan, (1, 1, 0, 0, 1));
    assert_eq!(runner.run_count(), 1);
    assert!(!diagnostics_trace_dir(fixture.app_data_dir()).exists());
    assert_eq!(report.items.len(), 1, "{report:#?}");
    let item = only_item(&report.items)?;
    assert_eq!(item.route.as_deref(), Some("provider_candidate"));
    assert_eq!(
        item.trace_retention,
        DecisionEvidenceTraceRetention::DiagnosticsMissing
    );
    assert!(item.trace_sequence.is_empty());
    Ok(())
}

fn copy_report_if_requested(
    name: &str,
    report: &morrow_lib::native_bridge::DecisionEvidenceReport,
) -> Result<(), String> {
    let Ok(dir) = std::env::var("MORROW_TASK5_DECISION_EVIDENCE_DIR") else {
        return Ok(());
    };
    let dir = std::path::PathBuf::from(dir);
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let serialized = serde_json::to_string(report).map_err(|error| error.to_string())?;
    fs::write(dir.join(name), serialized).map_err(|error| error.to_string())
}

fn only_item(items: &[DecisionEvidenceItem]) -> Result<&DecisionEvidenceItem, String> {
    match items {
        [item] => Ok(item),
        _ => Err(format!(
            "expected exactly one decision evidence item: {items:#?}"
        )),
    }
}
