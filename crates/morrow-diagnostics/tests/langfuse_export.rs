use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use morrow_diagnostics::export::write_trace_export_json_file;
use morrow_diagnostics::{
    export_langfuse_payload, prove_langfuse_backend_absent, write_langfuse_payload,
    LangfuseBackendConfig, LangfuseBackendStatus, LangfuseExportError, TraceRecord,
};

#[test]
fn langfuse_export_masks_payload_and_reports_drops() -> Result<(), TestError> {
    // Given: a local trace JSONL file with one valid record and one corrupt line.
    let fixture = TraceFixture::new("masking")?;
    let mut record = TraceRecord::sample_v1();
    record.span.provider_id = Some("fake-provider".to_owned());
    record.span.model_id = Some("offline-contract".to_owned());
    record.span.template_version = Some("prompt-v1".to_owned());
    fixture.write_trace_with_corrupt_line(&record)?;

    // When: the Langfuse adapter creates an offline payload capture.
    let payload = export_langfuse_payload(fixture.path())?;
    let json = serde_json::to_string_pretty(&payload)?;

    // Then: the capture is downstream-only, masked, and reports skipped corruption.
    assert_eq!(payload.ownership, "morrow_local_traces_are_canonical");
    assert_eq!(payload.export.records_read, 1);
    assert_eq!(payload.export.skipped_corrupt_lines, 1);
    assert!(json.contains("\"status\": \"masked\""));
    assert!(json.contains("\"dropped_categories\""));
    assert!(json.contains("\"native_identifier\""));
    assert_payload_has_no_forbidden_surface(&json);
    Ok(())
}

#[test]
fn langfuse_export_skips_trace_line_with_unsafe_metadata() -> Result<(), TestError> {
    // Given: a safe line plus typed trace lines carrying raw text in metadata and timestamps.
    let fixture = TraceFixture::new("reject-unsafe-metadata")?;
    let mut unsafe_record = TraceRecord::sample_v1();
    unsafe_record.span.provider_id = Some("MORROW_PRIVACY_CANARY_RAW_TEXT".to_owned());
    let mut unsafe_timestamp_record = TraceRecord::sample_v1();
    unsafe_timestamp_record.span.started_at = "MORROW_PRIVACY_CANARY_RAW_TEXT".to_owned();
    unsafe_timestamp_record.span.ended_at = Some("MORROW_PRIVACY_CANARY_RAW_TEXT".to_owned());
    fixture.write_three_traces(
        &TraceRecord::sample_v1(),
        &unsafe_record,
        &unsafe_timestamp_record,
    )?;

    // When: the Langfuse adapter creates the offline payload capture.
    let payload = export_langfuse_payload(fixture.path())?;
    let json = serde_json::to_string_pretty(&payload)?;

    // Then: the unsafe lines are counted as skipped and never emitted.
    assert_eq!(payload.export.records_read, 1);
    assert_eq!(payload.export.skipped_corrupt_lines, 2);
    assert!(!json.contains("MORROW_PRIVACY_CANARY_RAW_TEXT"));
    Ok(())
}

#[test]
fn langfuse_export_backend_absent_is_nonfatal() -> Result<(), TestError> {
    // Given: a trace file and a dummy Langfuse endpoint configuration.
    let fixture = TraceFixture::new("backend-absent")?;
    fixture.write_trace(&TraceRecord::sample_v1())?;
    let before = fs::read(fixture.path())?;
    let config = LangfuseBackendConfig {
        endpoint: Some("http://127.0.0.1:9".to_owned()),
        public_key: None,
        secret_key: None,
    };

    // When: backend absence is proven.
    let result = prove_langfuse_backend_absent(&config);

    // Then: an offline receipt is returned without network attempts or input mutation.
    let Err(LangfuseExportError::BackendUnavailable { receipt }) = result else {
        return Err(TestError::ExpectedBackendUnavailable);
    };
    assert_eq!(receipt.status, LangfuseBackendStatus::MissingCredentials);
    assert!(!receipt.network_attempted);
    assert_eq!(fs::read(fixture.path())?, before);
    let payload = export_langfuse_payload(fixture.path())?;
    assert_eq!(payload.export.records_read, 1);
    Ok(())
}

#[test]
fn langfuse_receipt_write_rejects_same_input_output_without_mutating_trace() -> Result<(), TestError>
{
    // Given: backend absence produces a receipt and the requested output is the input trace file.
    let fixture = TraceFixture::new("receipt-same-path")?;
    fixture.write_trace(&TraceRecord::sample_v1())?;
    let before = fs::read(fixture.path())?;
    let config = LangfuseBackendConfig {
        endpoint: Some("http://127.0.0.1:9".to_owned()),
        public_key: None,
        secret_key: None,
    };
    let Err(LangfuseExportError::BackendUnavailable { receipt }) =
        prove_langfuse_backend_absent(&config)
    else {
        return Err(TestError::ExpectedBackendUnavailable);
    };

    // When: the guarded write boundary is asked to write the receipt over the trace input.
    let result = write_trace_export_json_file(fixture.path(), fixture.path(), &receipt);

    // Then: the write is rejected before mutation and the trace bytes stay JSONL.
    assert!(matches!(
        result,
        Err(morrow_diagnostics::ExportReadError::OutputWouldOverwriteInput { .. })
    ));
    assert_eq!(fs::read(fixture.path())?, before);
    Ok(())
}

#[test]
fn langfuse_payload_write_rejects_same_input_output_without_mutating_trace() -> Result<(), TestError>
{
    // Given: a valid trace file is also passed as the requested payload output.
    let fixture = TraceFixture::new("payload-same-path")?;
    fixture.write_trace(&TraceRecord::sample_v1())?;
    let before = fs::read(fixture.path())?;

    // When: the Langfuse payload export tries to write to the input path.
    let result = write_langfuse_payload(fixture.path(), fixture.path());

    // Then: the payload write is rejected and the source trace is unchanged.
    assert!(matches!(
        result,
        Err(LangfuseExportError::Read(
            morrow_diagnostics::ExportReadError::OutputWouldOverwriteInput { .. }
        ))
    ));
    assert_eq!(fs::read(fixture.path())?, before);
    Ok(())
}

#[test]
fn langfuse_export_omits_local_only_trace_identifiers() -> Result<(), TestError> {
    // Given: a privacy-safe local trace containing hashed chat/message references.
    let fixture = TraceFixture::new("local-only-ids")?;
    let record = TraceRecord::sample_v1();
    let chat_hash = record.trace.chat_hash.clone();
    let message_hash = record.trace.message_hash.clone();
    fixture.write_trace(&record)?;

    // When: the local trace is transformed for Langfuse capture.
    let json = serde_json::to_string_pretty(&export_langfuse_payload(fixture.path())?)?;

    // Then: native chat/message references are not exported downstream.
    if let Some(hash) = chat_hash {
        assert!(!json.contains(&hash));
    }
    if let Some(hash) = message_hash {
        assert!(!json.contains(&hash));
    }
    Ok(())
}

fn assert_payload_has_no_forbidden_surface(json: &str) {
    for forbidden in [
        "raw_text",
        "prompt",
        "response",
        "raw_json",
        "embedding",
        "provider_json",
        "full_message",
        "raw_title",
        "title_text",
        "full_title",
        "unredacted_title",
    ] {
        assert!(!json.contains(forbidden), "{forbidden}");
    }
}

struct TraceFixture {
    dir: PathBuf,
    path: PathBuf,
}

impl TraceFixture {
    fn new(name: &str) -> Result<Self, TestError> {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let dir = std::env::temp_dir().join(format!("morrow-langfuse-{name}-{suffix}"));
        fs::create_dir_all(&dir)?;
        let path = dir.join("traces.jsonl");
        Ok(Self { dir, path })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn write_trace(&self, record: &TraceRecord) -> Result<(), TestError> {
        fs::write(&self.path, serde_json::to_string(record)? + "\n")?;
        Ok(())
    }

    fn write_trace_with_corrupt_line(&self, record: &TraceRecord) -> Result<(), TestError> {
        fs::write(
            &self.path,
            format!("{}\nnot-json\n", serde_json::to_string(record)?),
        )?;
        Ok(())
    }

    fn write_three_traces(
        &self,
        first: &TraceRecord,
        second: &TraceRecord,
        third: &TraceRecord,
    ) -> Result<(), TestError> {
        fs::write(
            &self.path,
            format!(
                "{}\n{}\n{}\n",
                serde_json::to_string(first)?,
                serde_json::to_string(second)?,
                serde_json::to_string(third)?
            ),
        )?;
        Ok(())
    }
}

impl Drop for TraceFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[derive(Debug, thiserror::Error)]
enum TestError {
    #[error("test io failed")]
    Io(#[from] std::io::Error),
    #[error("test clock failed")]
    Time(#[from] std::time::SystemTimeError),
    #[error("test serialization failed")]
    Serialization(#[from] serde_json::Error),
    #[error("langfuse export failed")]
    Export(#[from] LangfuseExportError),
    #[error("trace export write failed")]
    Write(#[from] morrow_diagnostics::ExportReadError),
    #[error("expected backend unavailable error")]
    ExpectedBackendUnavailable,
}
