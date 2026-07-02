use std::fs;

use morrow_diagnostics::{
    diagnostics_trace_dir, TraceComponent, TraceDecision, TraceOperation, TraceOutcome,
    TracePrivacyTier, TraceReader, TraceSchemaVersion,
};
use morrow_lib::native_bridge::CodexAuthStatus;

use super::support::{
    assert_counts, auth_readiness, candidate_json, scan_request_at, scan_result_counts,
    FakeCodexOutcome, RecordingCodexRunner, RejectingProposalAdapter, ScanFixture,
    FIXTURE_MESSAGE_TEXT, NATIVE_CHAT_ID, NATIVE_MESSAGE_ID, PRIVACY_CANARY,
};

#[test]
fn production_scan_writes_local_diagnostics_trace() -> Result<(), String> {
    // Given
    let fixture = ScanFixture::new("codex-trace")?;
    let mut request = scan_request_at(1_782_352_400)?;
    request.local_diagnostics_enabled = true;
    request.local_diagnostics_retention_days = 30;
    let provider_output = candidate_json();
    let runner =
        RecordingCodexRunner::new(vec![FakeCodexOutcome::WriteOutput(provider_output.clone())]);
    let adapter = RejectingProposalAdapter;

    // When
    let result = fixture.scan_with_request_and_app_data_dir(
        request,
        auth_readiness(CodexAuthStatus::LoggedInUsingChatGpt, true),
        &runner,
        &adapter,
    )?;

    // Then
    assert_counts(&result, (1, 1, 0, 0, 1));
    assert_eq!(runner.run_count(), 1);

    let traces_dir = diagnostics_trace_dir(fixture.app_data_dir());
    if !traces_dir.starts_with(fixture.app_data_dir()) {
        return Err(format!(
            "trace dir {} is not rooted in app data {}",
            traces_dir.display(),
            fixture.app_data_dir().display()
        ));
    }

    let trace_files = trace_jsonl_files(&traces_dir)?;
    if trace_files.is_empty() {
        return Err(format!(
            "enabled Codex scan wrote no traces in {}",
            traces_dir.display()
        ));
    }

    let read = TraceReader::new(&traces_dir)
        .read_all()
        .map_err(|error| error.to_string())?;
    assert_eq!(read.skipped_lines, 0);
    if read.records.is_empty() {
        return Err("TraceReader returned no records".to_owned());
    }

    assert!(read
        .records
        .iter()
        .all(|record| record.schema_version == TraceSchemaVersion::V1));
    assert!(read.records.iter().any(|record| {
        record.span.component == TraceComponent::Provider
            && record.span.operation == TraceOperation::ProviderRoute
            && record.span.decision == Some(TraceDecision::ProviderRoute)
    }));
    assert!(read.records.iter().any(|record| {
        record.span.component == TraceComponent::Provider
            && record.span.operation == TraceOperation::ProviderResult
            && record.span.reason_code.as_deref() == Some("provider_extract_success")
    }));
    assert!(read.records.iter().any(|record| {
        record.span.component == TraceComponent::Schema
            && record.span.operation == TraceOperation::SchemaValidation
            && record.span.decision == Some(TraceDecision::Candidate)
    }));
    assert!(read.records.iter().any(|record| {
        record.span.component == TraceComponent::Threshold
            && record.span.operation == TraceOperation::ThresholdDecision
            && record.span.decision == Some(TraceDecision::ConfidenceAccepted)
            && record.span.confidence_millis == Some(800)
    }));
    assert!(read.records.iter().any(|record| {
        record.span.component == TraceComponent::Outcome
            && record.span.operation == TraceOperation::OutcomeMaterialized
            && record.span.outcome == TraceOutcome::CandidateCreated
    }));
    assert!(read.records.iter().any(|record| {
        record.span.component == TraceComponent::Outcome
            && record.span.operation == TraceOperation::OutcomeMaterialized
            && record.span.privacy_tier == TracePrivacyTier::LocalPrivate
            && record.span.reason_code.as_deref() == Some("source_excerpt_included")
    }));

    let serialized = trace_files
        .iter()
        .map(fs::read_to_string)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?
        .join("\n");
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
    ] {
        if serialized.contains(forbidden) {
            return Err(format!(
                "trace JSONL leaked forbidden fixture string: {forbidden}"
            ));
        }
    }
    if let Ok(copy_path) = std::env::var("MORROW_TASK4_TRACE_COPY") {
        let copy_path = std::path::PathBuf::from(copy_path);
        if let Some(parent) = copy_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::copy(&trace_files[0], copy_path).map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[test]
fn production_scan_disabled_local_diagnostics_writes_no_trace() -> Result<(), String> {
    // Given
    let fixture = ScanFixture::new("codex-trace-disabled")?;
    let request = scan_request_at(1_782_352_400)?;
    let runner = RecordingCodexRunner::new(vec![FakeCodexOutcome::WriteOutput(
        candidate_json().to_owned(),
    )]);
    let adapter = RejectingProposalAdapter;

    // When
    let result = fixture.scan_with_request_and_app_data_dir(
        request,
        auth_readiness(CodexAuthStatus::LoggedInUsingChatGpt, true),
        &runner,
        &adapter,
    )?;

    // Then
    assert_counts(&result, (1, 1, 0, 0, 1));
    assert_eq!(runner.run_count(), 1);
    let traces_dir = diagnostics_trace_dir(fixture.app_data_dir());
    if traces_dir.exists() {
        let trace_files = trace_jsonl_files(&traces_dir)?;
        if !trace_files.is_empty() {
            return Err(format!(
                "disabled Codex diagnostics wrote trace files: {trace_files:?}"
            ));
        }
    }
    Ok(())
}

#[test]
fn production_trace_sink_failure_is_nonfatal() -> Result<(), String> {
    // Given
    let enabled_fixture = ScanFixture::new("codex-trace-enabled-counts")?;
    let disabled_fixture = ScanFixture::new("codex-trace-disabled-counts")?;
    let conflict_fixture = ScanFixture::new("codex-trace-sink-conflict")?;
    fs::create_dir_all(conflict_fixture.app_data_dir()).map_err(|error| error.to_string())?;
    fs::write(
        conflict_fixture.app_data_dir().join("diagnostics"),
        "regular file blocks diagnostics/traces creation",
    )
    .map_err(|error| error.to_string())?;

    let mut enabled_request = scan_request_at(1_782_352_400)?;
    enabled_request.local_diagnostics_enabled = true;
    enabled_request.local_diagnostics_retention_days = 30;
    let disabled_request = scan_request_at(1_782_352_400)?;
    let mut conflict_request = scan_request_at(1_782_352_400)?;
    conflict_request.local_diagnostics_enabled = true;
    conflict_request.local_diagnostics_retention_days = 30;

    let enabled_runner = RecordingCodexRunner::new(vec![FakeCodexOutcome::WriteOutput(
        candidate_json().to_owned(),
    )]);
    let disabled_runner = RecordingCodexRunner::new(vec![FakeCodexOutcome::WriteOutput(
        candidate_json().to_owned(),
    )]);
    let conflict_runner = RecordingCodexRunner::new(vec![FakeCodexOutcome::WriteOutput(
        candidate_json().to_owned(),
    )]);
    let adapter = RejectingProposalAdapter;

    // When
    let enabled_result = enabled_fixture.scan_with_request_and_app_data_dir(
        enabled_request,
        auth_readiness(CodexAuthStatus::LoggedInUsingChatGpt, true),
        &enabled_runner,
        &adapter,
    )?;
    let disabled_result = disabled_fixture.scan_with_request_and_app_data_dir(
        disabled_request,
        auth_readiness(CodexAuthStatus::LoggedInUsingChatGpt, true),
        &disabled_runner,
        &adapter,
    )?;
    let conflict_result = conflict_fixture.scan_with_request_and_app_data_dir(
        conflict_request,
        auth_readiness(CodexAuthStatus::LoggedInUsingChatGpt, true),
        &conflict_runner,
        &adapter,
    )?;

    // Then
    assert_eq!(enabled_runner.run_count(), 1);
    assert_eq!(disabled_runner.run_count(), 1);
    assert_eq!(conflict_runner.run_count(), 1);
    assert_eq!(
        scan_result_counts(&conflict_result),
        scan_result_counts(&enabled_result)
    );
    assert_eq!(
        scan_result_counts(&conflict_result),
        scan_result_counts(&disabled_result)
    );

    let enabled_trace_files =
        trace_jsonl_files(&diagnostics_trace_dir(enabled_fixture.app_data_dir()))?;
    if enabled_trace_files.is_empty() {
        return Err("enabled baseline wrote no trace files".to_owned());
    }

    let conflict_traces_dir = diagnostics_trace_dir(conflict_fixture.app_data_dir());
    if conflict_traces_dir.exists() {
        let conflict_trace_files = trace_jsonl_files(&conflict_traces_dir)?;
        if !conflict_trace_files.is_empty() {
            return Err(format!(
                "sink conflict unexpectedly wrote trace files: {conflict_trace_files:?}"
            ));
        }
    }
    Ok(())
}

fn trace_jsonl_files(traces_dir: &std::path::Path) -> Result<Vec<std::path::PathBuf>, String> {
    if !traces_dir.exists() {
        return Ok(Vec::new());
    }
    let mut paths = fs::read_dir(traces_dir)
        .map_err(|error| error.to_string())?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    paths.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "jsonl")
    });
    paths.sort();
    Ok(paths)
}
