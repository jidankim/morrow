use crate::validation::validate_text;
use crate::StorageError;

mod parser;

const SAFE_KEYS: &[&str] = &[
    "fixture",
    "source",
    "route",
    "reason_code",
    "label_source",
    "privacy_tier",
    "schema_version",
    "snapshot_schema_version",
    "trace_schema_version",
    "feedback_text_snapshots_enabled",
    "provider_id",
    "model_id",
    "template_version",
    "sender_signal_available",
    "context_window_available",
];

const FORBIDDEN_TERMS: &[&str] = &[
    "prompt",
    "response",
    "prompt_text",
    "response_text",
    "raw_text",
    "full_message",
    "message_history",
    "message_body",
    "raw_message",
    "private_message",
    "transcript",
    "conversation",
];

pub(super) fn validate_privacy_metadata_json(value: &str) -> Result<(), StorageError> {
    validate_text("privacy_metadata_json", value, 1_000)?;
    parser::parse_privacy_metadata_json(value)
}

fn validate_metadata_key(key: &str) -> Result<(), StorageError> {
    if has_forbidden_term(key) {
        return privacy_violation("privacy metadata key names private message content");
    }
    if SAFE_KEYS.contains(&key) {
        Ok(())
    } else {
        privacy_violation("privacy metadata key is not allowlisted")
    }
}

fn validate_metadata_value(value: &str) -> Result<(), StorageError> {
    if has_forbidden_term(value) {
        privacy_violation("privacy metadata value names private message content")
    } else {
        Ok(())
    }
}

fn normalize_token(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .flat_map(char::to_lowercase)
        .collect()
}

fn normalize_stem(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn has_forbidden_term(value: &str) -> bool {
    let token = normalize_token(value);
    let stem = normalize_stem(value);
    FORBIDDEN_TERMS
        .iter()
        .any(|term| token.contains(term) || stem.contains(&normalize_stem(term)))
}

fn privacy_violation<T>(reason: &str) -> Result<T, StorageError> {
    Err(StorageError::PrivacyViolation {
        reason: reason.to_owned(),
    })
}
