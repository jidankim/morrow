#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PrivacyError {
    #[error("forbidden trace field: {field}")]
    ForbiddenField { field: String },
    #[error("trace field is not privacy-allowlisted: {field}")]
    FieldNotAllowlisted { field: String },
    #[error("invalid sha256 hash in trace field {field}: {value}")]
    InvalidSha256Hash { field: String, value: String },
    #[error("invalid opaque trace identifier in field {field}: {value}")]
    InvalidOpaqueId { field: String, value: String },
    #[error("invalid privacy tier: {tier}")]
    InvalidPrivacyTier { tier: String },
    #[error("invalid title status: {status}")]
    InvalidTitleStatus { status: String },
    #[error("invalid safe metadata value in field {field}: {value}")]
    InvalidSafeMetadata { field: String, value: String },
    #[error("trace serialization failed")]
    Serialization,
}

mod rules;

use crate::trace::TraceRecord;
use rules::{
    is_opaque_id, is_safe_metadata_token, is_safe_timestamp, is_sha256_hash,
    reject_forbidden_field, require_allowlisted_field,
};

pub fn validate_trace_record_privacy(record: &TraceRecord) -> Result<(), PrivacyError> {
    let value = serde_json::to_value(record).map_err(|_| PrivacyError::Serialization)?;
    validate_trace_json_privacy(&value)
}

pub fn validate_trace_json_privacy(value: &serde_json::Value) -> Result<(), PrivacyError> {
    match value {
        serde_json::Value::Object(fields) => {
            for (field, child) in fields {
                reject_forbidden_field(field)?;
                require_allowlisted_field(field)?;
                validate_field_value(field, child)?;
                validate_trace_json_privacy(child)?;
            }
            Ok(())
        }
        serde_json::Value::Array(items) => {
            for item in items {
                validate_trace_json_privacy(item)?;
            }
            Ok(())
        }
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => Ok(()),
    }
}

fn validate_field_value(field: &str, value: &serde_json::Value) -> Result<(), PrivacyError> {
    match field {
        "chat_hash" | "message_hash" | "title_hash" => validate_optional_sha256_hash(field, value),
        "trace_id" => validate_opaque_id(field, value, "trace_"),
        "span_id" | "parent_span_id" => validate_opaque_id(field, value, "span_"),
        "privacy_tier" => validate_privacy_tier(value),
        "title_status" => validate_title_status(value),
        "started_at" | "ended_at" => {
            validate_optional_safe_field(field, value, is_safe_timestamp, field == "ended_at")
        }
        "provider_id" | "model_id" | "template_version" | "reason_code" | "classifier_stage"
        | "router_stage" | "replay_run_id" => {
            validate_optional_safe_field(field, value, is_safe_metadata_token, true)
        }
        _ => Ok(()),
    }
}

fn validate_optional_sha256_hash(
    field: &str,
    value: &serde_json::Value,
) -> Result<(), PrivacyError> {
    match value {
        serde_json::Value::Null => Ok(()),
        serde_json::Value::String(hash) if is_sha256_hash(hash) => Ok(()),
        serde_json::Value::String(hash) => Err(PrivacyError::InvalidSha256Hash {
            field: field.to_owned(),
            value: hash.to_owned(),
        }),
        _ => Err(PrivacyError::InvalidSha256Hash {
            field: field.to_owned(),
            value: value.to_string(),
        }),
    }
}

fn validate_opaque_id(
    field: &str,
    value: &serde_json::Value,
    prefix: &str,
) -> Result<(), PrivacyError> {
    if field == "parent_span_id" && value.is_null() {
        return Ok(());
    }
    let serde_json::Value::String(id) = value else {
        return Err(PrivacyError::InvalidOpaqueId {
            field: field.to_owned(),
            value: value.to_string(),
        });
    };
    if is_opaque_id(id, prefix) {
        return Ok(());
    }
    Err(PrivacyError::InvalidOpaqueId {
        field: field.to_owned(),
        value: id.to_owned(),
    })
}

fn validate_privacy_tier(value: &serde_json::Value) -> Result<(), PrivacyError> {
    match value.as_str() {
        Some("public" | "internal_metadata" | "hashed_identifier" | "local_private") => Ok(()),
        Some(tier) => Err(PrivacyError::InvalidPrivacyTier {
            tier: tier.to_owned(),
        }),
        None => Err(PrivacyError::InvalidPrivacyTier {
            tier: value.to_string(),
        }),
    }
}

fn validate_title_status(value: &serde_json::Value) -> Result<(), PrivacyError> {
    match value {
        serde_json::Value::Null => Ok(()),
        serde_json::Value::String(status) if matches!(status.as_str(), "hashed" | "absent") => {
            Ok(())
        }
        serde_json::Value::String(status) => Err(PrivacyError::InvalidTitleStatus {
            status: status.to_owned(),
        }),
        _ => Err(PrivacyError::InvalidTitleStatus {
            status: value.to_string(),
        }),
    }
}

fn validate_optional_safe_field(
    field: &str,
    value: &serde_json::Value,
    is_safe: fn(&str) -> bool,
    allow_null: bool,
) -> Result<(), PrivacyError> {
    match value {
        serde_json::Value::Null if allow_null => Ok(()),
        serde_json::Value::String(metadata) if is_safe(metadata) => Ok(()),
        serde_json::Value::String(metadata) => Err(PrivacyError::InvalidSafeMetadata {
            field: field.to_owned(),
            value: metadata.to_owned(),
        }),
        _ => Err(PrivacyError::InvalidSafeMetadata {
            field: field.to_owned(),
            value: value.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests;
