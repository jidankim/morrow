use crate::feedback_eval::FeedbackRecordMeta;
use crate::sqlite_cli::SqliteValue;
use crate::{CandidateId, FeedbackLabelType, StorageError};

pub(super) fn push_common_params<'a>(
    params: &mut Vec<SqliteValue<'a>>,
    meta: &'a FeedbackRecordMeta,
) -> Result<(), StorageError> {
    push_common_body_params(params, meta)?;
    params.extend([
        SqliteValue::Integer(meta.created_at),
        optional_i64(meta.expires_at),
    ]);
    Ok(())
}

pub(super) fn push_common_body_params<'a>(
    params: &mut Vec<SqliteValue<'a>>,
    meta: &'a FeedbackRecordMeta,
) -> Result<(), StorageError> {
    super::feedback_eval_validation::validate_meta(meta)?;
    params.extend([
        SqliteValue::Integer(meta.schema_version),
        SqliteValue::Text(meta.subject_type.as_str()),
        SqliteValue::Text(&meta.subject_id),
        optional_text(meta.candidate_id.as_ref().map(CandidateId::as_str)),
        SqliteValue::Text(&meta.chat_guid),
        SqliteValue::Text(&meta.anchor_message_guid),
        optional_text(meta.diagnostics.trace_id.as_deref()),
        optional_text(meta.diagnostics.span_id.as_deref()),
        optional_text(meta.diagnostics.parent_span_id.as_deref()),
        optional_text(meta.diagnostics.chat_hash.as_deref()),
        optional_text(meta.diagnostics.message_hash.as_deref()),
        optional_i64(meta.provider_model_prompt_version_id),
        SqliteValue::Text(meta.source_excerpt_policy.as_str()),
        SqliteValue::Text(meta.label_source.as_str()),
        SqliteValue::Text(meta.privacy_tier.as_str()),
        SqliteValue::Text(&meta.privacy_metadata_json),
    ]);
    Ok(())
}

pub(super) fn push_record_times<'a>(params: &mut Vec<SqliteValue<'a>>, meta: &FeedbackRecordMeta) {
    params.extend([
        SqliteValue::Integer(meta.created_at),
        optional_i64(meta.expires_at),
    ]);
}

pub(super) fn optional_text(value: Option<&str>) -> SqliteValue<'_> {
    match value {
        Some(value) => SqliteValue::Text(value),
        None => SqliteValue::Null,
    }
}

pub(super) fn optional_label_type(value: Option<FeedbackLabelType>) -> SqliteValue<'static> {
    match value {
        Some(value) => SqliteValue::Text(value.as_str()),
        None => SqliteValue::Null,
    }
}

pub(super) fn optional_i64(value: Option<i64>) -> SqliteValue<'static> {
    match value {
        Some(value) => SqliteValue::Integer(value),
        None => SqliteValue::Null,
    }
}

pub(super) const fn bool_int(value: bool) -> i64 {
    match value {
        true => 1,
        false => 0,
    }
}
