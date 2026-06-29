use serde_json::json;

use super::{
    rules::FORBIDDEN_FIELD_PARTS, validate_trace_json_privacy, validate_trace_record_privacy,
};
use crate::trace::TraceRecord;

const PRIVACY_CANARY: &str = "MORROW_PRIVACY_CANARY_RAW_TEXT";
type MetadataCanaryCase = (&'static str, fn(&mut TraceRecord));

#[test]
fn privacy_rejects_forbidden_trace_fields() {
    // Given: a trace-shaped payload with a raw-content field.
    let value = json!({
        "schema_version": "trace.v1",
        "span": {
            "operation": "provider_result",
            "raw_text": "private message body"
        }
    });

    // When: the privacy scanner validates the payload.
    let result = validate_trace_json_privacy(&value);

    // Then: the scanner rejects the forbidden field name.
    assert!(result.is_err());
}

#[test]
fn privacy_rejects_all_forbidden_trace_field_names() {
    // Given: every forbidden raw-content field token from the trace contract.
    // When / Then: each field is rejected by the scanner.
    for &field in FORBIDDEN_FIELD_PARTS {
        let value = json!({ field: "private" });
        assert!(validate_trace_json_privacy(&value).is_err(), "{field}");
    }
}

#[test]
fn privacy_allows_opaque_title_metadata_when_allowlisted() {
    // Given: title metadata represented only as an opaque hash and status.
    let value = json!({
        "title_hash": "sha256:bd3a8371554e90455a68feeb46d376b24f44e4e81e1c71225ddd1671e4fd3e06",
        "title_status": "hashed"
    });

    // When: the privacy scanner validates the payload.
    let result = validate_trace_json_privacy(&value);

    // Then: allowlisted opaque title metadata is accepted.
    assert_eq!(result, Ok(()));
}

#[test]
fn privacy_allows_safe_free_form_trace_metadata() {
    // Given: a trace record using the safe metadata identities expected by exports.
    let mut record = TraceRecord::sample_v1();
    record.span.provider_id = Some("fake-provider".to_owned());
    record.span.model_id = Some("offline-contract".to_owned());
    record.span.template_version = Some("prompt-v1".to_owned());
    record.span.reason_code = Some("deterministic_stop:no_scheduling_signal".to_owned());
    record.span.classifier_stage = Some("reserved_classifier".to_owned());
    record.span.router_stage = Some("reserved_router".to_owned());
    record.span.replay_run_id = Some("replay-local-fixture".to_owned());

    // When: the privacy scanner validates the full typed trace record.
    let result = validate_trace_record_privacy(&record);

    // Then: safe metadata identities remain exportable.
    assert_eq!(result, Ok(()));
}

#[test]
fn privacy_rejects_canary_in_free_form_trace_metadata() {
    // Given / When / Then: every allowlisted free-form metadata field rejects raw text.
    for (field, inject_canary) in unsafe_metadata_cases() {
        let mut record = TraceRecord::sample_v1();
        inject_canary(&mut record);
        assert!(validate_trace_record_privacy(&record).is_err(), "{field}");
    }
}

#[test]
fn privacy_rejects_canary_in_trace_timestamps() {
    // Given / When / Then: exported timestamp fields reject raw private text.
    for (field, inject_canary) in unsafe_timestamp_cases() {
        let mut record = TraceRecord::sample_v1();
        inject_canary(&mut record);
        assert!(validate_trace_record_privacy(&record).is_err(), "{field}");
    }
}

#[test]
fn privacy_accepts_known_safe_trace_timestamps() {
    // Given: the timestamp forms currently emitted by fixtures and message traces.
    let accepted_values = [
        ("2026-06-28T05:00:00Z", Some("2026-06-28T05:00:00Z")),
        (
            "message_timestamp:1782352400",
            Some("message_timestamp:1782352401"),
        ),
        ("2026-06-28T05:00:00Z", None),
    ];

    // When / Then: existing safe timestamp contracts remain accepted.
    for (started_at, ended_at) in accepted_values {
        let mut record = TraceRecord::sample_v1();
        record.span.started_at = started_at.to_owned();
        record.span.ended_at = ended_at.map(str::to_owned);
        assert_eq!(
            validate_trace_record_privacy(&record),
            Ok(()),
            "{started_at}"
        );
    }
}

#[test]
fn privacy_rejects_prose_and_jsonish_trace_timestamps() {
    // Given: raw prose and JSON-ish values in timestamp positions.
    let rejected_values = [
        "meet tomorrow at the private clinic",
        "{\"timestamp\":\"2026-06-28T05:00:00Z\"}",
        "2026-06-28 05:00:00",
    ];

    // When / Then: timestamp fields remain structural metadata, not raw text carriers.
    for rejected in rejected_values {
        let mut record = TraceRecord::sample_v1();
        record.span.started_at = rejected.to_owned();
        assert!(
            validate_trace_record_privacy(&record).is_err(),
            "{rejected}"
        );
    }
}

#[test]
fn privacy_rejects_raw_and_malformed_hash_and_opaque_values() {
    // Given: allowlisted hash and opaque ID fields carrying raw or malformed values.
    let rejected_values = [
        ("chat_hash", "chat-guid-raw"),
        ("message_hash", "message-guid-raw"),
        ("title_hash", "Team Sync"),
        ("chat_hash", "sha256:nothex"),
        ("trace_id", "trace_chat-guid-raw"),
        ("span_id", "span_message-guid-raw"),
        ("parent_span_id", "span_nothex"),
    ];

    // When / Then: each bad value is rejected despite the allowlisted field name.
    for (field, raw_value) in rejected_values {
        let value = json!({ field: raw_value });
        assert!(validate_trace_json_privacy(&value).is_err(), "{field}");
    }
}

#[test]
fn privacy_rejects_unknown_privacy_tier() {
    // Given: a trace payload with a tier outside the privacy contract.
    let value = json!({ "privacy_tier": "raw_content" });

    // When: the privacy scanner validates the payload.
    let result = validate_trace_json_privacy(&value);

    // Then: the scanner rejects the unknown tier.
    assert!(result.is_err());
}

#[test]
fn privacy_rejects_fields_missing_allowlist_entries() {
    // Given: a field that is not part of the v1 trace allowlist.
    let value = json!({ "new_unreviewed_field": "metadata" });

    // When: the privacy scanner validates the payload.
    let result = validate_trace_json_privacy(&value);

    // Then: schema additions must update the privacy allowlist first.
    assert!(result.is_err());
}

fn unsafe_metadata_cases() -> [MetadataCanaryCase; 7] {
    [
        ("provider_id", set_provider_id_canary),
        ("model_id", set_model_id_canary),
        ("template_version", set_template_version_canary),
        ("reason_code", set_reason_code_canary),
        ("classifier_stage", set_classifier_stage_canary),
        ("router_stage", set_router_stage_canary),
        ("replay_run_id", set_replay_run_id_canary),
    ]
}

fn unsafe_timestamp_cases() -> [MetadataCanaryCase; 2] {
    [
        ("started_at", set_started_at_canary),
        ("ended_at", set_ended_at_canary),
    ]
}

fn set_started_at_canary(record: &mut TraceRecord) {
    record.span.started_at = PRIVACY_CANARY.to_owned();
}

fn set_ended_at_canary(record: &mut TraceRecord) {
    record.span.ended_at = Some(PRIVACY_CANARY.to_owned());
}

fn set_provider_id_canary(record: &mut TraceRecord) {
    record.span.provider_id = Some(PRIVACY_CANARY.to_owned());
}

fn set_model_id_canary(record: &mut TraceRecord) {
    record.span.model_id = Some(PRIVACY_CANARY.to_owned());
}

fn set_template_version_canary(record: &mut TraceRecord) {
    record.span.template_version = Some(PRIVACY_CANARY.to_owned());
}

fn set_reason_code_canary(record: &mut TraceRecord) {
    record.span.reason_code = Some(PRIVACY_CANARY.to_owned());
}

fn set_classifier_stage_canary(record: &mut TraceRecord) {
    record.span.classifier_stage = Some(PRIVACY_CANARY.to_owned());
}

fn set_router_stage_canary(record: &mut TraceRecord) {
    record.span.router_stage = Some(PRIVACY_CANARY.to_owned());
}

fn set_replay_run_id_canary(record: &mut TraceRecord) {
    record.span.replay_run_id = Some(PRIVACY_CANARY.to_owned());
}
