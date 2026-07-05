use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use morrow_diagnostics::{
    validate_trace_json_privacy, validate_trace_record_privacy, JsonlTraceSink, TraceComponent,
    TraceDecision, TraceIds, TraceOperation, TraceOutcome, TracePrivacyTier, TraceReader,
    TraceRecord, TraceSchemaVersion, TraceSpan,
};

const CANARY_LABEL: &str = "MORROW_PRIVACY_CANARY_RAW_CORRECTION";

#[test]
fn user_correction_trace_serializes_sanitized_vocabulary() -> Result<(), String> {
    // Given: the canonical Phase 4 human approval correction trace record.
    let record = user_correction_record();

    // When: the record is serialized as canonical JSONL.
    let serialized = serde_json::to_string(&record).map_err(|error| error.to_string())? + "\n";

    // Then: the sanitized correction vocabulary matches the committed fixture.
    assert_eq!(
        serialized,
        include_str!("../fixtures/human_approval_correction_trace_v1_snapshot.jsonl")
    );
    assert_serialized_vocabulary(&record)?;
    println!("component=correction");
    println!("operation=user_correction");
    println!("decision=user_corrected");
    Ok(())
}

#[test]
fn user_correction_trace_survives_jsonl_sink_readback() -> Result<(), String> {
    // Given: a local JSONL trace sink and sanitized user correction trace record.
    let temp_dir = temp_evidence_dir("human-approval-correction-readback")?;
    let sink = JsonlTraceSink::new(&temp_dir).map_err(|error| error.to_string())?;
    let record = user_correction_record();

    // When: the record is written through the privacy-validating sink.
    sink.append(&record).map_err(|error| error.to_string())?;
    let readback = TraceReader::new(sink.traces_dir())
        .read_all()
        .map_err(|error| error.to_string())?;

    // Then: JSONL readback returns the same sanitized correction record.
    assert_eq!(readback.skipped_lines, 0);
    assert_eq!(readback.records, vec![record]);
    Ok(())
}

#[test]
fn user_correction_trace_passes_privacy_rules() -> Result<(), String> {
    // Given: a correction record carrying only opaque IDs, hashes, and safe metadata.
    let record = user_correction_record();

    // When / Then: the typed record passes the diagnostics privacy allowlist.
    validate_trace_record_privacy(&record).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn user_correction_trace_rejects_raw_correction_canaries() -> Result<(), String> {
    // Given: a trace-shaped canary with raw corrected title/message fields.
    let raw = include_str!("../fixtures/human_approval_correction_trace_canary_rejected.jsonl");
    let value: serde_json::Value = serde_json::from_str(raw).map_err(|error| error.to_string())?;
    let span = value
        .pointer("/span")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| "missing span object in canary fixture".to_owned())?;
    assert!(span.contains_key("raw_title"));
    assert!(span.contains_key("full_message"));
    assert!(span.contains_key("native_identifier"));

    // When: serde and the privacy scanner inspect the untrusted boundary payload.
    let typed_result = serde_json::from_str::<TraceRecord>(raw);
    let privacy_result = validate_trace_json_privacy(&value);

    // Then: unknown fields and raw correction canaries are rejected before persistence.
    assert!(typed_result.is_err());
    assert!(privacy_result.is_err());
    println!("privacy_canary_rejection={CANARY_LABEL}");
    println!("typed_validation=non_ok");
    println!("privacy_validation=non_ok");
    Ok(())
}

fn user_correction_record() -> TraceRecord {
    TraceRecord {
        schema_version: TraceSchemaVersion::V1,
        trace: TraceIds {
            trace_id: "trace_00000000000000000000000000000041".to_owned(),
            span_id: "span_00000000000000000000000000000041".to_owned(),
            parent_span_id: None,
            chat_hash: Some(
                "sha256:66e0bc3220b7dd3d0651965d244ebba7f5a8ae571be6874570b58495cdf26d85"
                    .to_owned(),
            ),
            message_hash: Some(
                "sha256:d9cee5362324ef2404962c149f0c564c7f6f009fe91ba5985cc5341a79a348de"
                    .to_owned(),
            ),
        },
        span: TraceSpan {
            component: TraceComponent::Correction,
            operation: TraceOperation::UserCorrection,
            decision: Some(TraceDecision::UserCorrected),
            outcome: TraceOutcome::Noop,
            started_at: "2026-06-29T05:00:01Z".to_owned(),
            ended_at: Some("2026-06-29T05:00:01Z".to_owned()),
            provider_id: None,
            model_id: None,
            template_version: None,
            reason_code: Some("user_correction_applied".to_owned()),
            confidence_millis: None,
            title_hash: Some(
                "sha256:b12b98bbd7274cb93dee7cb18ca8a9a0247a007c99480b46f1879cc211459c0d"
                    .to_owned(),
            ),
            title_status: Some("hashed".to_owned()),
            privacy_tier: TracePrivacyTier::HashedIdentifier,
            classifier_stage: None,
            router_stage: None,
            ood_score_millis: None,
            replay_run_id: Some("phase4-correction-local-001".to_owned()),
        },
    }
}

fn assert_serialized_vocabulary(record: &TraceRecord) -> Result<(), String> {
    let value = serde_json::to_value(record).map_err(|error| error.to_string())?;
    assert_eq!(
        value
            .pointer("/span/component")
            .and_then(serde_json::Value::as_str),
        Some("correction")
    );
    assert_eq!(
        value
            .pointer("/span/operation")
            .and_then(serde_json::Value::as_str),
        Some("user_correction")
    );
    assert_eq!(
        value
            .pointer("/span/decision")
            .and_then(serde_json::Value::as_str),
        Some("user_corrected")
    );
    Ok(())
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
