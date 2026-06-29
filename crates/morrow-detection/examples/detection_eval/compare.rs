use std::collections::BTreeMap;
use std::error::Error;

use morrow_diagnostics::{
    TraceComponent, TraceDecision, TraceOperation, TraceOutcome, TracePrivacyTier, TraceRecord,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::fixture::Scenario;
use super::model::Mismatch;

const CANARY: &str = "MORROW_PRIVACY_CANARY_RAW_TEXT";
const FORBIDDEN_TRACE_PARTS: &[&str] = &[
    "raw_text",
    "response",
    "raw_json",
    "provider_json",
    "full_message",
    "raw_title",
    "title_text",
    "full_title",
    "unredacted_title",
];

pub fn count_mismatches(
    scenario: &Scenario,
    candidate_count: usize,
    quiet_count: usize,
    provider_calls: usize,
) -> Vec<Mismatch> {
    let mut mismatches = Vec::new();
    push_count(
        "candidate_count",
        scenario.expected_candidate_count,
        candidate_count,
        &mut mismatches,
    );
    push_count(
        "quiet_count",
        scenario.expected_quiet_count,
        quiet_count,
        &mut mismatches,
    );
    push_count(
        "provider_call_count",
        scenario.expected_provider_calls,
        provider_calls,
        &mut mismatches,
    );
    mismatches
}

pub fn compare_trace(
    scenario: &Scenario,
    records: &[TraceRecord],
    mismatches: &mut Vec<Mismatch>,
    confusion: &mut BTreeMap<String, usize>,
) {
    push_count(
        "trace_length",
        scenario.expected_trace.len(),
        records.len(),
        mismatches,
    );
    for (step, (expected, record)) in scenario.expected_trace.iter().zip(records).enumerate() {
        compare_field(
            step,
            "component",
            &expected.component,
            &record.span.component,
            mismatches,
            confusion,
        );
        compare_field(
            step,
            "operation",
            &expected.operation,
            &record.span.operation,
            mismatches,
            confusion,
        );
        compare_field(
            step,
            "decision",
            &expected.decision,
            &record.span.decision,
            mismatches,
            confusion,
        );
        compare_field(
            step,
            "outcome",
            &expected.outcome,
            &record.span.outcome,
            mismatches,
            confusion,
        );
        compare_field(
            step,
            "reason_code",
            &expected.reason_code,
            &record.span.reason_code,
            mismatches,
            confusion,
        );
        compare_field(
            step,
            "privacy_tier",
            &expected.privacy_tier,
            &record.span.privacy_tier,
            mismatches,
            confusion,
        );
    }
}

pub fn trace_leaked_private_input(
    scenario: &Scenario,
    records: &[TraceRecord],
) -> Result<bool, Box<dyn Error>> {
    let serialized = serde_json::to_string(records)?;
    let message_leak = scenario
        .messages
        .iter()
        .any(|message| serialized.contains(&message.excerpt));
    let provider_leak = scenario
        .provider_response
        .as_ref()
        .is_some_and(|provider_response| serialized.contains(provider_response));
    let forbidden_leak = FORBIDDEN_TRACE_PARTS
        .iter()
        .any(|forbidden| serialized.contains(forbidden));
    Ok(message_leak || provider_leak || forbidden_leak || serialized.contains(CANARY))
}

fn compare_field<T: PartialEq + Serialize>(
    step: usize,
    field: &'static str,
    expected: &T,
    actual: &T,
    mismatches: &mut Vec<Mismatch>,
    confusion: &mut BTreeMap<String, usize>,
) {
    if expected == actual {
        return;
    }
    let expected_value = sanitized_mismatch_value(field, json_value(expected));
    let actual_value = sanitized_mismatch_value(field, json_value(actual));
    if matches!(field, "operation" | "decision" | "reason_code") {
        let key = format!("{field}:{}=>{}", expected_value, actual_value);
        *confusion.entry(key).or_insert(0) += 1;
    }
    mismatches.push(Mismatch {
        step: Some(step),
        field,
        expected: expected_value,
        actual: actual_value,
    });
}

fn push_count(field: &'static str, expected: usize, actual: usize, mismatches: &mut Vec<Mismatch>) {
    if expected != actual {
        mismatches.push(Mismatch {
            step: None,
            field,
            expected: json_value(&expected),
            actual: json_value(&actual),
        });
    }
}

fn json_value<T: Serialize>(value: &T) -> Value {
    match serde_json::to_value(value) {
        Ok(value) => value,
        Err(_) => Value::String("<unserializable>".to_owned()),
    }
}

fn sanitized_mismatch_value(field: &str, value: Value) -> Value {
    match field {
        "candidate_count" | "quiet_count" | "provider_call_count" | "trace_length" => value,
        "component" => trace_vocabulary_value::<TraceComponent>(value),
        "operation" => trace_vocabulary_value::<TraceOperation>(value),
        "decision" => trace_vocabulary_value::<TraceDecision>(value),
        "outcome" => trace_vocabulary_value::<TraceOutcome>(value),
        "privacy_tier" => trace_vocabulary_value::<TracePrivacyTier>(value),
        "reason_code" => reason_code_value(value),
        _ => hashed_value(value),
    }
}

fn trace_vocabulary_value<T: DeserializeOwned>(value: Value) -> Value {
    if value.is_null() || serde_json::from_value::<T>(value.clone()).is_ok() {
        value
    } else {
        hashed_value(value)
    }
}

fn reason_code_value(value: Value) -> Value {
    match value.as_str() {
        Some(text) if is_safe_reason_code(text) => value,
        None if value.is_null() => value,
        _ => hashed_value(value),
    }
}

fn is_safe_reason_code(text: &str) -> bool {
    matches!(
        text,
        "candidate_materialized"
            | "confidence_below_threshold"
            | "confidence_meets_threshold"
            | "deterministic_stop:invalid_evidence"
            | "deterministic_stop:no_scheduling_signal"
            | "deterministic_stop:past_or_invalid_time"
            | "deterministic_stop:unsupported_broad_content"
            | "parser_candidate"
            | "parser_provider_route"
            | "parser_provider_time_conflict"
            | "provider_extract_success"
            | "provider_hallucinated_evidence"
            | "provider_invalid_json"
            | "provider_route"
            | "provider_schema_accepted"
            | "provider_schema_rejected"
            | "provider_unavailable"
            | "source_excerpt_hidden"
            | "source_excerpt_included"
    )
}

fn hashed_value(value: Value) -> Value {
    let canonical = match serde_json::to_string(&value) {
        Ok(serialized) => serialized,
        Err(_) => "<unserializable>".to_owned(),
    };
    let digest = Sha256::digest(canonical.as_bytes());
    let hex = format!("{digest:x}");
    let short_hash: String = hex.chars().take(16).collect();
    Value::String(format!("<redacted:sha256:{short_hash}>"))
}
