use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use morrow_diagnostics::{
    validate_trace_json_privacy, validate_trace_record_privacy, JsonlTraceSink, TraceOperation,
    TraceReader, TraceRecord,
};

const REQUIRED_OPERATION_NAMES: &[&str] = &[
    "candidate_superseded",
    "candidate_rescheduled",
    "candidate_cancelled",
    "calendar_dry_run",
    "calendar_commit_idempotency",
    "replay_run",
];

#[test]
fn lifecycle_trace_records_cover_phase3_operations_when_serialized() -> Result<(), String> {
    // Given: the Phase 3 lifecycle/replay trace fixture records.
    let records = lifecycle_records()?;

    // When: the records are serialized through the typed trace schema.
    let operation_names = records
        .iter()
        .map(serialized_operation_name)
        .collect::<Result<Vec<_>, _>>()?;

    // Then: all required operations are present with canonical snake_case names.
    assert_eq!(operation_names, REQUIRED_OPERATION_NAMES);
    Ok(())
}

#[test]
fn lifecycle_trace_excludes_user_correction_from_phase3_records() -> Result<(), String> {
    // Given: the Phase 3 lifecycle/replay trace fixture records.
    let records = lifecycle_records()?;

    // When: the reserved user correction operation is requested as a lifecycle record.
    let user_correction_record =
        TraceRecord::sample_lifecycle_replay_v1(TraceOperation::UserCorrection);

    // Then: Phase 3 remains scoped to lifecycle/replay operations only.
    assert_eq!(user_correction_record, None);
    assert!(!records
        .iter()
        .any(|record| record.span.operation == TraceOperation::UserCorrection));
    Ok(())
}

#[test]
fn lifecycle_trace_fixture_matches_serializer() -> Result<(), String> {
    // Given: the canonical Phase 3 lifecycle/replay trace fixture records.
    let records = lifecycle_records()?;

    // When: the records are serialized as canonical JSONL.
    let serialized = records
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?
        .join("\n")
        + "\n";

    // Then: the bytes match the committed lifecycle trace snapshot.
    assert_eq!(
        serialized,
        include_str!("../fixtures/lifecycle_trace_v1_snapshot.jsonl")
    );
    Ok(())
}

#[test]
fn lifecycle_trace_records_survive_jsonl_sink_readback() -> Result<(), String> {
    // Given: a local JSONL trace sink and the lifecycle/replay fixture records.
    let temp_dir = temp_evidence_dir("lifecycle-readback")?;
    let sink = JsonlTraceSink::new(&temp_dir).map_err(|error| error.to_string())?;
    let records = lifecycle_records()?;

    // When: each record is written through the privacy-validating sink.
    for record in &records {
        sink.append(record).map_err(|error| error.to_string())?;
    }
    let readback = TraceReader::new(sink.traces_dir())
        .read_all()
        .map_err(|error| error.to_string())?;

    // Then: JSONL readback returns the same sanitized trace records.
    assert_eq!(readback.skipped_lines, 0);
    assert_eq!(readback.records, records);
    Ok(())
}

#[test]
fn lifecycle_trace_records_pass_privacy_rules() -> Result<(), String> {
    // Given: lifecycle/replay records carrying only opaque IDs and safe metadata.
    let records = lifecycle_records()?;

    // When / Then: every typed record passes the diagnostics privacy allowlist.
    for record in records {
        validate_trace_record_privacy(&record).map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[test]
fn lifecycle_trace_rejects_unknown_and_raw_content_fields() -> Result<(), String> {
    // Given: a trace-shaped raw-content fixture with an unknown forbidden field.
    let raw = include_str!("../fixtures/lifecycle_trace_canary_rejected.jsonl");
    let value: serde_json::Value = serde_json::from_str(raw).map_err(|error| error.to_string())?;

    // When: serde and the privacy scanner inspect the boundary payload.
    let typed_result = serde_json::from_str::<TraceRecord>(raw);
    let privacy_result = validate_trace_json_privacy(&value);

    // Then: unknown fields and raw-content canaries are rejected before persistence.
    assert!(typed_result.is_err());
    assert!(privacy_result.is_err());
    Ok(())
}

fn lifecycle_records() -> Result<Vec<TraceRecord>, String> {
    [
        TraceOperation::CandidateSuperseded,
        TraceOperation::CandidateRescheduled,
        TraceOperation::CandidateCancelled,
        TraceOperation::CalendarDryRun,
        TraceOperation::CalendarCommitIdempotency,
        TraceOperation::ReplayRun,
    ]
    .into_iter()
    .map(|operation| {
        TraceRecord::sample_lifecycle_replay_v1(operation)
            .ok_or_else(|| format!("missing lifecycle trace fixture for {operation:?}"))
    })
    .collect()
}

fn serialized_operation_name(record: &TraceRecord) -> Result<String, String> {
    let value = serde_json::to_value(record).map_err(|error| error.to_string())?;
    value
        .pointer("/span/operation")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("missing serialized operation in {value}"))
}

fn temp_evidence_dir(name: &str) -> Result<PathBuf, String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "morrow-diagnostics-{name}-{}-{nanos}",
        std::process::id()
    ));
    create_clean_dir(&dir)?;
    Ok(dir)
}

fn create_clean_dir(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(path).map_err(|error| error.to_string())
}
