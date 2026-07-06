pub mod export;
pub mod privacy;
pub mod sink;
pub mod trace;
mod trace_trajectory;

pub use export::{
    export_langfuse_payload, prove_langfuse_backend_absent, read_trace_input, write_json_file,
    write_langfuse_payload, ExportReadError, LangfuseBackendConfig, LangfuseBackendReceipt,
    LangfuseBackendStatus, LangfuseExportError, LangfuseExportReceipt, LangfusePayload,
};
pub use privacy::{validate_trace_json_privacy, validate_trace_record_privacy, PrivacyError};
pub use sink::{
    diagnostics_trace_dir, JsonlTraceSink, JsonlTraceSinkConfig, TraceReadResult, TraceReader,
    TraceSinkError,
};
pub use trace::{
    NoopTraceRecorder, OpaqueIdError, TraceComponent, TraceDecision, TraceIds, TraceOperation,
    TraceOutcome, TracePrivacyTier, TraceRecord, TraceRecorder, TraceRecorderError,
    TraceSchemaVersion, TraceSpan,
};
pub use trace_trajectory::{
    TrajectoryAnchorError, TrajectoryAnchorReport, TrajectoryAnchorStep, TrajectoryAnchorStepInput,
};

#[cfg(test)]
mod tests {
    use super::{
        validate_trace_record_privacy, NoopTraceRecorder, OpaqueIdError, TraceIds, TraceOperation,
        TraceRecord, TraceRecorder, TraceSchemaVersion, TraceSpan,
    };

    #[test]
    fn trace_schema_fixture_matches_serializer() -> Result<(), serde_json::Error> {
        // Given: the canonical v1 trace fixture record.
        let record = TraceRecord::sample_v1();

        // When: the record is serialized as JSONL.
        let serialized = serde_json::to_string(&record)? + "\n";

        // Then: the bytes match the committed schema snapshot.
        assert_eq!(
            serialized,
            include_str!("../fixtures/trace_v1_snapshot.jsonl")
        );
        Ok(())
    }

    #[test]
    fn trace_schema_sample_passes_privacy_allowlist() {
        // Given: the canonical v1 trace fixture record.
        let record = TraceRecord::sample_v1();

        // When: the privacy allowlist scans the serialized record.
        let result = validate_trace_record_privacy(&record);

        // Then: every emitted field has an explicit privacy decision.
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn trace_schema_ids_are_random_and_message_ids_are_hashed() -> Result<(), OpaqueIdError> {
        // Given: two trace ID requests for the same local identifiers.
        let first = TraceIds::random(Some("chat-guid"), Some("message-guid"))?;
        let second = TraceIds::random(Some("chat-guid"), Some("message-guid"))?;

        // When / Then: trace IDs are opaque and non-deterministic, while native IDs are hashed.
        assert_ne!(first.trace_id, second.trace_id);
        assert_ne!(first.span_id, second.span_id);
        assert_ne!(first.chat_hash.as_deref(), Some("chat-guid"));
        assert_ne!(first.message_hash.as_deref(), Some("message-guid"));
        assert_eq!(first.chat_hash, second.chat_hash);
        assert_eq!(first.message_hash, second.message_hash);
        Ok(())
    }

    #[test]
    fn trace_schema_rejects_unknown_privacy_tier() {
        // Given: serialized trace data with an unknown privacy tier.
        let raw = include_str!("../fixtures/trace_v1_snapshot.jsonl")
            .replace("hashed_identifier", "raw_content");

        // When: serde parses the boundary type.
        let result = serde_json::from_str::<TraceRecord>(&raw);

        // Then: unknown privacy tiers fail at the parse boundary.
        assert!(result.is_err());
    }

    #[test]
    fn trace_schema_includes_reserved_lifecycle_operation() {
        // Given: the canonical v1 sample span.
        let span = TraceSpan::sample_v1();

        // When / Then: a reserved lifecycle diagnostic operation is represented in schema v1.
        assert_eq!(span.operation, TraceOperation::PollEmpty);
    }

    #[test]
    fn noop_trace_recorder_accepts_trace_records_without_side_effects() {
        // Given: a privacy-safe trace record and noop recorder.
        let recorder = NoopTraceRecorder;
        let record = TraceRecord::sample_v1();

        // When: the record is submitted.
        let result = recorder.record(&record);

        // Then: noop recording succeeds without persistence.
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn trace_schema_version_is_v1() {
        // Given: the canonical sample record.
        let record = TraceRecord::sample_v1();

        // When / Then: the fixture pins schema v1.
        assert_eq!(record.schema_version, TraceSchemaVersion::V1);
    }
}
