use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

use super::{default_phoenix_export_path, export_phoenix_payload, PhoenixExportConfig};
use crate::trace::TraceRecord;

#[test]
fn phoenix_export_writes_sanitized_openinference_payload() -> Result<(), TestError> {
    // Given: a local JSONL trace file with one privacy-safe trace record.
    let work_dir = unique_temp_dir("payload")?;
    let input = work_dir.join("traces.jsonl");
    let output = work_dir.join("phoenix-payload.json");
    fs::write(
        &input,
        serde_json::to_string(&TraceRecord::sample_v1())? + "\n",
    )?;

    // When: the Phoenix adapter exports a payload capture.
    let receipt = export_phoenix_payload(&PhoenixExportConfig {
        input_path: &input,
        output_path: &output,
    })?;

    // Then: the payload contains viewer span fields and only sanitized metadata.
    assert_eq!(receipt.span_count, 1);
    assert_eq!(receipt.skipped_corrupt_lines, 0);
    let payload: serde_json::Value = serde_json::from_str(&fs::read_to_string(&output)?)?;
    assert_eq!(field(&payload, "adapter")?, "phoenix_openinference_json");
    let source = field(&payload, "source")?;
    assert_eq!(
        field(source, "source_of_truth")?,
        "morrow_local_trace_jsonl"
    );
    assert_eq!(field(source, "trace_schema_version")?, "v1");
    let span = first_item(field(&payload, "spans")?)?;
    let attributes = field(span, "attributes")?;
    assert_eq!(
        field(span, "span_id")?,
        "span_52efdebab8574a5fa6f290831a786e86"
    );
    assert_eq!(field(span, "parent_span_id")?, &serde_json::Value::Null);
    assert_eq!(field(attributes, "morrow.component")?, "lifecycle");
    assert_eq!(field(attributes, "morrow.operation")?, "poll_empty");
    assert_eq!(field(attributes, "morrow.outcome")?, "noop");
    assert_eq!(
        field(attributes, "morrow.reason_code")?,
        "no_messages_ready"
    );
    assert_eq!(field(attributes, "morrow.trace_schema_version")?, "v1");
    assert_eq!(field(attributes, "morrow.privacy.title_status")?, "hashed");
    assert!(!payload_contains_forbidden_value(&payload));
    Ok(())
}

#[test]
fn phoenix_export_rejects_local_only_fields() -> Result<(), TestError> {
    // Given: one valid trace line and one JSON line containing local-only raw fields.
    let work_dir = unique_temp_dir("reject-local-only")?;
    let input = work_dir.join("traces.jsonl");
    let output = work_dir.join("phoenix-payload.json");
    let leaked = json!({
        "schema_version": "v1",
        "raw_text": "MORROW_PRIVACY_CANARY_RAW_TEXT",
        "prompt": "private prompt",
        "embedding": [1, 2, 3]
    });
    fs::write(
        &input,
        format!(
            "{}\n{}\n",
            serde_json::to_string(&TraceRecord::sample_v1())?,
            leaked
        ),
    )?;

    // When: the Phoenix adapter reads the JSONL input.
    let receipt = export_phoenix_payload(&PhoenixExportConfig {
        input_path: &input,
        output_path: &output,
    })?;

    // Then: the local-only line is rejected and the payload contains no leaked values.
    assert_eq!(receipt.span_count, 1);
    assert_eq!(receipt.skipped_corrupt_lines, 1);
    let payload = fs::read_to_string(&output)?;
    assert!(!payload.contains("MORROW_PRIVACY_CANARY_RAW_TEXT"));
    assert!(!payload.contains("private prompt"));
    assert!(!payload.contains("[1,2,3]"));
    Ok(())
}

#[test]
fn phoenix_export_skips_trace_line_with_unsafe_metadata() -> Result<(), TestError> {
    // Given: a safe line plus typed trace lines carrying raw text in metadata and timestamps.
    let work_dir = unique_temp_dir("reject-unsafe-metadata")?;
    let input = work_dir.join("traces.jsonl");
    let output = work_dir.join("phoenix-payload.json");
    let mut unsafe_record = TraceRecord::sample_v1();
    unsafe_record.span.provider_id = Some("MORROW_PRIVACY_CANARY_RAW_TEXT".to_owned());
    let mut unsafe_timestamp_record = TraceRecord::sample_v1();
    unsafe_timestamp_record.span.started_at = "MORROW_PRIVACY_CANARY_RAW_TEXT".to_owned();
    unsafe_timestamp_record.span.ended_at = Some("MORROW_PRIVACY_CANARY_RAW_TEXT".to_owned());
    fs::write(
        &input,
        format!(
            "{}\n{}\n{}\n",
            serde_json::to_string(&TraceRecord::sample_v1())?,
            serde_json::to_string(&unsafe_record)?,
            serde_json::to_string(&unsafe_timestamp_record)?
        ),
    )?;

    // When: Phoenix exports the JSONL input.
    let receipt = export_phoenix_payload(&PhoenixExportConfig {
        input_path: &input,
        output_path: &output,
    })?;

    // Then: the unsafe lines are counted as skipped and never emitted.
    assert_eq!(receipt.span_count, 1);
    assert_eq!(receipt.skipped_corrupt_lines, 2);
    let payload = fs::read_to_string(&output)?;
    assert!(!payload.contains("MORROW_PRIVACY_CANARY_RAW_TEXT"));
    Ok(())
}

#[test]
fn phoenix_export_rejects_same_input_output_without_mutating_trace() -> Result<(), TestError> {
    // Given: a valid trace file is also passed as the requested Phoenix output.
    let work_dir = unique_temp_dir("same-path")?;
    let input = work_dir.join("traces.jsonl");
    fs::write(
        &input,
        serde_json::to_string(&TraceRecord::sample_v1())? + "\n",
    )?;
    let before = fs::read(&input)?;

    // When: Phoenix export is asked to write the payload over the input trace.
    let result = export_phoenix_payload(&PhoenixExportConfig {
        input_path: &input,
        output_path: &input,
    });

    // Then: the export is rejected and the source trace bytes are unchanged.
    assert!(matches!(
        result,
        Err(super::PhoenixExportError::ExportIo(
            crate::export::ExportReadError::OutputWouldOverwriteInput { .. }
        ))
    ));
    assert_eq!(fs::read(&input)?, before);
    Ok(())
}

#[test]
fn phoenix_export_reports_opaque_trace_and_span_identifier_policy() -> Result<(), TestError> {
    // Given: a local trace file with a standard Phoenix output path.
    let work_dir = unique_temp_dir("identifier-policy")?;
    let input = work_dir.join("traces.jsonl");
    let output = work_dir.join("phoenix-payload.json");
    fs::write(
        &input,
        serde_json::to_string(&TraceRecord::sample_v1())? + "\n",
    )?;

    // When: the Phoenix adapter writes the payload capture.
    export_phoenix_payload(&PhoenixExportConfig {
        input_path: &input,
        output_path: &output,
    })?;

    // Then: privacy wording matches the trace_id and span_id fields emitted in the payload.
    let payload: serde_json::Value = serde_json::from_str(&fs::read_to_string(&output)?)?;
    assert_eq!(
        field(field(&payload, "privacy")?, "identifier_policy")?,
        "opaque_trace_and_span_ids_only"
    );
    Ok(())
}

#[test]
fn phoenix_export_default_path_is_diagnostics_export_capture() {
    // Given: an app data root.
    let app_data = Path::new("/tmp/morrow-app-data");

    // When: the adapter derives its default output path.
    let path = default_phoenix_export_path(app_data);

    // Then: payload captures are under diagnostics/exports/phoenix.
    assert_eq!(
        path,
        app_data
            .join("diagnostics")
            .join("exports")
            .join("phoenix")
            .join("phoenix-payload.json")
    );
}

fn payload_contains_forbidden_value(payload: &serde_json::Value) -> bool {
    payload.to_string().contains("team sync")
        || payload.to_string().contains("chat-guid")
        || payload.to_string().contains("message-guid")
}

fn field<'a>(
    value: &'a serde_json::Value,
    key: &'static str,
) -> Result<&'a serde_json::Value, TestError> {
    value.get(key).ok_or(TestError::MissingJsonField(key))
}

fn first_item(value: &serde_json::Value) -> Result<&serde_json::Value, TestError> {
    let Some(items) = value.as_array() else {
        return Err(TestError::MissingJsonField("array"));
    };
    items
        .first()
        .ok_or(TestError::MissingJsonField("first_item"))
}

fn unique_temp_dir(name: &str) -> Result<PathBuf, TestError> {
    let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!("morrow-phoenix-export-{name}-{suffix}"));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[derive(Debug, thiserror::Error)]
enum TestError {
    #[error("test io failed")]
    Io(#[from] std::io::Error),
    #[error("test clock failed")]
    Time(#[from] std::time::SystemTimeError),
    #[error("test serialization failed")]
    Json(#[from] serde_json::Error),
    #[error("phoenix export failed")]
    Export(#[from] super::PhoenixExportError),
    #[error("missing json field {0}")]
    MissingJsonField(&'static str),
}
