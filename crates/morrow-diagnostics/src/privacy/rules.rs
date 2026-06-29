use super::PrivacyError;

pub(super) const FORBIDDEN_FIELD_PARTS: &[&str] = &[
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
];

const ALLOWED_FIELDS: &[&str] = &[
    "schema_version",
    "trace",
    "trace_id",
    "span_id",
    "parent_span_id",
    "chat_hash",
    "message_hash",
    "span",
    "component",
    "operation",
    "decision",
    "outcome",
    "started_at",
    "ended_at",
    "provider_id",
    "model_id",
    "template_version",
    "reason_code",
    "confidence_millis",
    "title_hash",
    "title_status",
    "privacy_tier",
    "classifier_stage",
    "router_stage",
    "ood_score_millis",
    "replay_run_id",
];

const SHA256_PREFIX: &str = "sha256:";

pub(super) fn reject_forbidden_field(field: &str) -> Result<(), PrivacyError> {
    if FORBIDDEN_FIELD_PARTS
        .iter()
        .any(|forbidden| field.contains(forbidden))
    {
        return Err(PrivacyError::ForbiddenField {
            field: field.to_owned(),
        });
    }
    Ok(())
}

pub(super) fn require_allowlisted_field(field: &str) -> Result<(), PrivacyError> {
    if ALLOWED_FIELDS.contains(&field) {
        return Ok(());
    }
    Err(PrivacyError::FieldNotAllowlisted {
        field: field.to_owned(),
    })
}

pub(super) fn is_sha256_hash(value: &str) -> bool {
    let Some(digest) = value.strip_prefix(SHA256_PREFIX) else {
        return false;
    };
    digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn is_opaque_id(value: &str, prefix: &str) -> bool {
    let Some(token) = value.strip_prefix(prefix) else {
        return false;
    };
    token.len() == 32 && token.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn is_safe_metadata_token(value: &str) -> bool {
    if value.is_empty() || value.len() > 128 {
        return false;
    }
    let Some(first) = value.bytes().next() else {
        return false;
    };
    let Some(last) = value.bytes().last() else {
        return false;
    };
    is_metadata_alphanumeric(first)
        && is_metadata_alphanumeric(last)
        && value
            .bytes()
            .all(|byte| is_metadata_alphanumeric(byte) || matches!(byte, b'-' | b'_' | b':' | b'.'))
}

pub(super) fn is_safe_timestamp(value: &str) -> bool {
    if let Some(digits) = value.strip_prefix("message_timestamp:") {
        return !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit());
    }
    value.len() == 20
        && value.bytes().enumerate().all(|(index, byte)| match index {
            4 | 7 => byte == b'-',
            10 => byte == b'T',
            13 | 16 => byte == b':',
            19 => byte == b'Z',
            _ => byte.is_ascii_digit(),
        })
}

const fn is_metadata_alphanumeric(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit()
}
