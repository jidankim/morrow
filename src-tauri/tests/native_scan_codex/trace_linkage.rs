use std::collections::HashSet;

use morrow_diagnostics::{diagnostics_trace_dir, TraceReader};
use morrow_lib::native_bridge::CodexAuthStatus;

use super::support::{
    assert_counts, auth_readiness, candidate_json, scan_request_at, FakeCodexOutcome,
    RecordingCodexRunner, RejectingProposalAdapter, ScanFixture,
};

#[test]
fn production_trace_linkage_reaches_feedback_eval_metadata() -> Result<(), String> {
    // Given
    let fixture = ScanFixture::new("codex-trace-linkage")?;
    let mut request = scan_request_at(1_782_352_400)?;
    request.local_diagnostics_enabled = true;
    request.local_diagnostics_retention_days = 30;
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
    let read = TraceReader::new(&traces_dir)
        .read_all()
        .map_err(|error| error.to_string())?;
    assert_eq!(read.skipped_lines, 0);
    let persisted_links = read
        .records
        .iter()
        .map(|record| (record.trace.trace_id.clone(), record.trace.span_id.clone()))
        .collect::<HashSet<_>>();
    if persisted_links.is_empty() {
        return Err(format!(
            "enabled Codex scan wrote no readable trace records in {}",
            traces_dir.display()
        ));
    }

    let cases = fixture.eval_cases()?;
    let linked_case = cases
        .iter()
        .find(|case| {
            case.snapshot.route.as_deref() == Some("provider_candidate")
                && case.snapshot.meta.diagnostics.trace_id.is_some()
                && case.snapshot.meta.diagnostics.span_id.is_some()
        })
        .ok_or_else(|| {
            format!("no provider-backed eval case carried diagnostics linkage: {cases:#?}")
        })?;
    let diagnostics = &linked_case.snapshot.meta.diagnostics;
    let trace_id = diagnostics
        .trace_id
        .as_deref()
        .ok_or_else(|| "linked eval case omitted trace id".to_owned())?;
    let span_id = diagnostics
        .span_id
        .as_deref()
        .ok_or_else(|| "linked eval case omitted span id".to_owned())?;
    assert_privacy_safe_trace_id(trace_id, "trace id")?;
    assert_privacy_safe_span_id(span_id, "span id")?;
    if let Some(parent_span_id) = diagnostics.parent_span_id.as_deref() {
        assert_privacy_safe_span_id(parent_span_id, "parent span id")?;
    }
    assert_privacy_safe_hash(diagnostics.chat_hash.as_deref(), "chat hash")?;
    assert_privacy_safe_hash(diagnostics.message_hash.as_deref(), "message hash")?;

    if !persisted_links.contains(&(trace_id.to_owned(), span_id.to_owned())) {
        return Err(format!(
            "eval case diagnostics link {trace_id}/{span_id} was absent from persisted JSONL traces"
        ));
    }
    Ok(())
}

fn assert_privacy_safe_trace_id(value: &str, field: &'static str) -> Result<(), String> {
    assert_prefixed_hex(value, "trace_", field)
}

fn assert_privacy_safe_span_id(value: &str, field: &'static str) -> Result<(), String> {
    assert_prefixed_hex(value, "span_", field)
}

fn assert_prefixed_hex(value: &str, prefix: &str, field: &'static str) -> Result<(), String> {
    let Some(hex) = value.strip_prefix(prefix) else {
        return Err(format!("{field} is not a privacy-safe opaque id: {value}"));
    };
    if !hex.is_empty() && hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(format!("{field} has non-hex opaque id bytes: {value}"))
    }
}

fn assert_privacy_safe_hash(value: Option<&str>, field: &'static str) -> Result<(), String> {
    let Some(value) = value else {
        return Err(format!("linked eval case omitted {field}"));
    };
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(format!("{field} is not a privacy-safe hash: {value}"));
    };
    if hex.len() == 64 && hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(format!("{field} has invalid sha256 payload: {value}"))
    }
}
