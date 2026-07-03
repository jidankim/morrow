use std::fs;

use morrow_lib::native_bridge::{
    load_decision_evidence_at, CodexAuthStatus, DecisionEvidenceItem,
    DecisionEvidenceTraceRetention, LoadDecisionEvidenceRequest, NativeDecisionEvidenceSubjectType,
};
use serde_json::{json, Value};

use super::super::support::{
    assert_counts, auth_readiness, scan_request_at, FakeCodexOutcome, RecordingCodexRunner,
    RejectingProposalAdapter, ScanFixture,
};

#[test]
fn decision_evidence_retains_quiet_provider_rejection_trace() -> Result<(), String> {
    // Given
    let fixture = ScanFixture::new("codex-decision-evidence-quiet-retained")?;
    let mut request = scan_request_at(1_782_352_400)?;
    request.local_diagnostics_enabled = true;
    request.local_diagnostics_retention_days = 30;
    let runner = RecordingCodexRunner::new(vec![FakeCodexOutcome::WriteOutput(
        low_confidence_candidate_json(),
    )]);
    let adapter = RejectingProposalAdapter;
    let scan = fixture.scan_with_request_and_app_data_dir(
        request,
        auth_readiness(CodexAuthStatus::LoggedInUsingChatGpt, true),
        &runner,
        &adapter,
    )?;
    assert_counts(&scan, (0, 0, 1, 0, 0));

    // When
    let report = load_decision_evidence_at(
        fixture.store_path(),
        fixture.app_data_dir(),
        LoadDecisionEvidenceRequest {
            limit: 10,
            created_candidate_ids: Vec::new(),
        },
    )?;
    copy_report_if_requested("quiet-provider-rejection.json", &report)?;

    // Then
    assert_eq!(report.items.len(), 1, "{report:#?}");
    assert_eq!(report.skipped_trace_line_count, 0);
    let item = only_item(&report.items)?;
    assert_eq!(
        item.subject_type,
        NativeDecisionEvidenceSubjectType::QuietLog
    );
    assert_eq!(item.candidate_id, None);
    assert_eq!(item.route.as_deref(), Some("provider_rejection"));
    assert_eq!(
        item.reason_code.as_deref(),
        Some("confidence_below_threshold")
    );
    assert_eq!(item.confidence_millis, Some(320));
    assert_eq!(item.label_type, "proposal_outcome");
    assert_eq!(item.label_value, "unknown");
    assert_eq!(item.provider_diagnostic, None);
    assert_eq!(provider_diagnostic_from_item(item)?, None);
    assert!(item.has_diagnostics_hashes);
    assert_eq!(
        item.trace_retention,
        DecisionEvidenceTraceRetention::Retained
    );
    assert_trace_sequence(
        item,
        &[
            "parser_decision",
            "provider_route",
            "provider_result",
            "schema_validation",
            "threshold_decision",
            "outcome_materialized",
            "outcome_materialized",
        ],
        "quiet_logged",
    )
}

#[test]
fn decision_evidence_surfaces_provider_unavailable_diagnostic() -> Result<(), String> {
    // Given
    let fixture = ScanFixture::new("codex-decision-evidence-provider-unavailable")?;
    let mut request = scan_request_at(1_782_352_400)?;
    request.local_diagnostics_enabled = true;
    request.local_diagnostics_retention_days = 30;
    let runner = RecordingCodexRunner::new(vec![FakeCodexOutcome::TimedOut]);
    let adapter = RejectingProposalAdapter;
    let scan = fixture.scan_with_request_and_app_data_dir(
        request,
        auth_readiness(CodexAuthStatus::LoggedInUsingChatGpt, true),
        &runner,
        &adapter,
    )?;
    assert_counts(&scan, (0, 0, 1, 0, 0));

    // When
    let report = load_decision_evidence_at(
        fixture.store_path(),
        fixture.app_data_dir(),
        LoadDecisionEvidenceRequest {
            limit: 10,
            created_candidate_ids: Vec::new(),
        },
    )?;
    copy_report_if_requested("native-decision-evidence.json", &report)?;

    // Then
    assert_eq!(runner.run_count(), 1);
    assert_eq!(report.items.len(), 1, "{report:#?}");
    assert_eq!(report.skipped_trace_line_count, 0);
    let item = only_item(&report.items)?;
    assert_eq!(
        item.subject_type,
        NativeDecisionEvidenceSubjectType::QuietLog
    );
    assert_eq!(item.candidate_id, None);
    assert_eq!(item.route.as_deref(), Some("provider_rejection"));
    assert_eq!(item.reason_code.as_deref(), Some("provider_unavailable"));
    assert_eq!(item.confidence_millis, None);
    assert_eq!(
        item.provider_diagnostic.as_deref(),
        Some("codex provider command timed out")
    );
    assert_eq!(
        provider_diagnostic_from_item(item)?,
        Some("codex provider command timed out".to_owned())
    );
    assert!(item.has_diagnostics_hashes);
    assert_eq!(
        item.trace_retention,
        DecisionEvidenceTraceRetention::Retained
    );
    assert_trace_sequence(
        item,
        &[
            "parser_decision",
            "provider_route",
            "provider_result",
            "outcome_materialized",
            "outcome_materialized",
        ],
        "quiet_logged",
    )
}

fn low_confidence_candidate_json() -> String {
    json!({
        "kind": "calendar_event",
        "title": "Low confidence provider title",
        "confidence_millis": 320,
        "normalized_time": "2026-06-26T15:00:00[Asia/Seoul]",
        "anchor_evidence_id": "evidence://selected/0",
        "evidence_ids": ["evidence://selected/0"],
    })
    .to_string()
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

fn provider_diagnostic_from_item(item: &DecisionEvidenceItem) -> Result<Option<String>, String> {
    let value = serde_json::to_value(item).map_err(|error| error.to_string())?;
    match value.get("providerDiagnostic") {
        Some(Value::String(diagnostic)) => Ok(Some(diagnostic.clone())),
        Some(Value::Null) | None => Ok(None),
        Some(other) => Err(format!(
            "providerDiagnostic must be string or null, got {other:#?}"
        )),
    }
}

fn assert_trace_sequence(
    item: &DecisionEvidenceItem,
    expected_operations: &[&str],
    expected_outcome: &str,
) -> Result<(), String> {
    let operations = item
        .trace_sequence
        .iter()
        .map(|step| step.operation.as_str())
        .collect::<Vec<_>>();
    assert_eq!(operations, expected_operations);
    if !item
        .trace_sequence
        .iter()
        .any(|step| step.outcome == expected_outcome)
    {
        return Err(format!(
            "retained item omitted expected outcome {expected_outcome}: {item:#?}"
        ));
    }
    Ok(())
}
