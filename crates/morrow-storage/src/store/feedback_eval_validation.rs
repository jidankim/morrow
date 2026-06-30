use crate::feedback_eval::{
    EvalResult, EvalRun, FeatureSnapshot, FeedbackLabelValue, FeedbackRecordMeta,
    FEEDBACK_EVAL_SCHEMA_VERSION, REDACTED_SOURCE_EXCERPT,
};
use crate::validation::validate_text;
use crate::{DiagnosticsTraceLinkage, FeedbackSourceExcerptPolicy, StorageError};

pub(super) fn validate_meta(meta: &FeedbackRecordMeta) -> Result<(), StorageError> {
    if meta.schema_version != FEEDBACK_EVAL_SCHEMA_VERSION {
        return invalid(
            "schema_version",
            "must equal feedback/eval schema version 1",
        );
    }
    validate_text("subject_id", &meta.subject_id, 240)?;
    validate_text("chat_guid", &meta.chat_guid, 240)?;
    validate_text("anchor_message_guid", &meta.anchor_message_guid, 240)?;
    super::feedback_privacy_metadata::validate_privacy_metadata_json(&meta.privacy_metadata_json)?;
    validate_diagnostics(&meta.diagnostics)?;
    validate_expiry(meta.created_at, meta.expires_at)
}

pub(super) fn validate_snapshot(snapshot: &FeatureSnapshot) -> Result<(), StorageError> {
    validate_key("snapshot_key", &snapshot.snapshot_key)?;
    validate_optional_bounded("route", snapshot.route.as_deref(), 80)?;
    validate_optional_bounded("reason_code", snapshot.reason_code.as_deref(), 160)?;
    validate_optional_bounded("tapback_signal", snapshot.tapback_signal.as_deref(), 80)?;
    validate_optional_non_negative("participant_count", snapshot.participant_count)?;
    if let Some(confidence) = snapshot.confidence_millis {
        validate_millis("confidence_millis", confidence)?;
    }
    validate_snapshot_excerpt(snapshot)
}

pub(super) fn validate_eval_run(run: &EvalRun) -> Result<(), StorageError> {
    validate_key("run_key", &run.run_key)?;
    if run.schema_version != FEEDBACK_EVAL_SCHEMA_VERSION {
        return invalid(
            "schema_version",
            "must equal feedback/eval schema version 1",
        );
    }
    validate_optional_bounded("report_path", run.report_path.as_deref(), 500)?;
    validate_json_object("skip_reasons_json", &run.skip_reasons_json)?;
    validate_json_object("confusion_counts_json", &run.confusion_counts_json)?;
    validate_non_negative("cases_evaluated", run.cases_evaluated)?;
    validate_non_negative("cases_skipped", run.cases_skipped)?;
    validate_non_negative("quiet_log_count", run.quiet_log_count)?;
    validate_non_negative("provider_failure_count", run.provider_failure_count)?;
    validate_millis("approval_kept_rate_millis", run.approval_kept_rate_millis)?;
    validate_millis(
        "observed_rejection_rate_millis",
        run.observed_rejection_rate_millis,
    )?;
    validate_millis(
        "accepted_visible_ratio_millis",
        run.accepted_visible_ratio_millis,
    )?;
    validate_expiry(run.started_at, run.finished_at)
}

pub(super) fn validate_eval_result(result: &EvalResult) -> Result<(), StorageError> {
    validate_key("result_key", &result.result_key)?;
    validate_text("snapshot_key", &result.snapshot_key, 240)?;
    validate_text("label_key", &result.label_key, 240)?;
    FeedbackLabelValue::parse(
        result.expected_label_type,
        &result.expected_label_value,
        "expected_label_value",
    )?;
    if let Some(actual_label_type) = result.actual_label_type {
        let Some(actual_label_value) = result.actual_label_value.as_deref() else {
            return invalid(
                "actual_label_value",
                "is required when actual_label_type is set",
            );
        };
        FeedbackLabelValue::parse(actual_label_type, actual_label_value, "actual_label_value")?;
    }
    if result.actual_label_type.is_none() && result.actual_label_value.is_some() {
        return invalid(
            "actual_label_type",
            "is required when actual_label_value is set",
        );
    }
    validate_optional_bounded("skip_reason", result.skip_reason.as_deref(), 240)
}

pub(super) fn validate_key(field: &'static str, value: &str) -> Result<(), StorageError> {
    validate_text(field, value, 240)
}

fn validate_snapshot_excerpt(snapshot: &FeatureSnapshot) -> Result<(), StorageError> {
    let Some(excerpt) = snapshot.excerpt.as_deref() else {
        return Ok(());
    };
    validate_text("excerpt", excerpt, 280)?;
    if excerpt.lines().count() > 3 {
        return Err(StorageError::PrivacyViolation {
            reason: "feature snapshots accept short excerpts only".to_owned(),
        });
    }
    match snapshot.meta.source_excerpt_policy {
        FeedbackSourceExcerptPolicy::Include => Ok(()),
        FeedbackSourceExcerptPolicy::Hide if excerpt == REDACTED_SOURCE_EXCERPT => Ok(()),
        FeedbackSourceExcerptPolicy::Hide => Err(StorageError::PrivacyViolation {
            reason: "hidden source excerpt policy only accepts the redacted placeholder".to_owned(),
        }),
    }
}

fn validate_diagnostics(diagnostics: &DiagnosticsTraceLinkage) -> Result<(), StorageError> {
    validate_opaque_id(
        "diagnostics_trace_id",
        diagnostics.trace_id.as_deref(),
        "trace_",
    )?;
    validate_opaque_id(
        "diagnostics_span_id",
        diagnostics.span_id.as_deref(),
        "span_",
    )?;
    validate_opaque_id(
        "diagnostics_parent_span_id",
        diagnostics.parent_span_id.as_deref(),
        "span_",
    )?;
    validate_hash("diagnostics_chat_hash", diagnostics.chat_hash.as_deref())?;
    validate_hash(
        "diagnostics_message_hash",
        diagnostics.message_hash.as_deref(),
    )
}

fn validate_opaque_id(
    field: &'static str,
    value: Option<&str>,
    prefix: &str,
) -> Result<(), StorageError> {
    let Some(value) = value else {
        return Ok(());
    };
    validate_text(field, value, 80)?;
    if value.strip_prefix(prefix).is_some_and(is_hex) {
        Ok(())
    } else {
        invalid(field, "must be a copied privacy-safe diagnostics opaque id")
    }
}

fn validate_hash(field: &'static str, value: Option<&str>) -> Result<(), StorageError> {
    let Some(value) = value else {
        return Ok(());
    };
    validate_text(field, value, 80)?;
    if value.strip_prefix("sha256:").is_some_and(is_sha256_hex) {
        Ok(())
    } else {
        invalid(
            field,
            "must be a copied privacy-safe diagnostics sha256 hash",
        )
    }
}

fn is_hex(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && is_hex(value)
}

fn validate_json_object(field: &'static str, value: &str) -> Result<(), StorageError> {
    validate_text(field, value, 1_000)?;
    let trimmed = value.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        Ok(())
    } else {
        invalid(field, "must be a JSON object")
    }
}

fn validate_optional_bounded(
    field: &'static str,
    value: Option<&str>,
    max_len: usize,
) -> Result<(), StorageError> {
    match value {
        Some(value) => validate_text(field, value, max_len),
        None => Ok(()),
    }
}

fn validate_optional_non_negative(
    field: &'static str,
    value: Option<i64>,
) -> Result<(), StorageError> {
    match value {
        Some(value) => validate_non_negative(field, value),
        None => Ok(()),
    }
}

fn validate_non_negative(field: &'static str, value: i64) -> Result<(), StorageError> {
    if value >= 0 {
        Ok(())
    } else {
        invalid(field, "must be non-negative")
    }
}

fn validate_millis(field: &'static str, value: i64) -> Result<(), StorageError> {
    if (0..=1000).contains(&value) {
        Ok(())
    } else {
        invalid(field, "must be between 0 and 1000")
    }
}

fn validate_expiry(created_at: i64, expires_at: Option<i64>) -> Result<(), StorageError> {
    match expires_at {
        Some(expires_at) if expires_at <= created_at => {
            invalid("expires_at", "must be later than created_at")
        }
        Some(_) | None => Ok(()),
    }
}

fn invalid<T>(field: &'static str, reason: &str) -> Result<T, StorageError> {
    Err(StorageError::InvalidInput {
        field,
        reason: reason.to_owned(),
    })
}
