use crate::feedback_eval::{
    EvalCase, FeatureSnapshot, FeedbackLabelValue, FeedbackRecordMeta, Label,
};
use crate::sqlite_cli::row_value;
use crate::{
    CandidateId, DiagnosticsTraceLinkage, FeedbackLabelSource, FeedbackLabelType,
    FeedbackPrivacyTier, FeedbackSourceExcerptPolicy, FeedbackSubjectType, StorageError, Store,
};

impl Store {
    pub fn eval_cases(&self) -> Result<Vec<EvalCase>, StorageError> {
        self.sqlite
            .query_rows(
                "SELECT
                   s.snapshot_key, s.schema_version, s.subject_type, s.subject_id,
                   s.candidate_id, s.chat_guid, s.anchor_message_guid, s.diagnostics_trace_id,
                   s.diagnostics_span_id, s.diagnostics_parent_span_id, s.diagnostics_chat_hash,
                   s.diagnostics_message_hash, s.provider_model_prompt_version_id,
                   s.source_excerpt_policy, s.label_source, s.privacy_tier,
                   s.privacy_metadata_json, s.route, s.reason_code, s.confidence_millis,
                   s.participant_count, s.tapback_signal, s.sender_signal_available,
                   s.context_window_available, s.excerpt, s.created_at, s.expires_at,
                   l.label_key, l.label_type, l.label_value
                 FROM feature_snapshots s
                 JOIN labels l
                   ON l.subject_type = s.subject_type
                  AND l.subject_id = s.subject_id
                 ORDER BY s.id, l.id;",
            )?
            .into_iter()
            .map(row_to_eval_case)
            .collect()
    }
}

fn row_to_eval_case(row: Vec<String>) -> Result<EvalCase, StorageError> {
    let meta = FeedbackRecordMeta {
        schema_version: row_i64(&row, 1, "snapshot.schema_version")?,
        subject_type: FeedbackSubjectType::parse(row_value(&row, 2, "snapshot.subject_type")?)?,
        subject_id: row_value(&row, 3, "snapshot.subject_id")?.to_owned(),
        candidate_id: optional_candidate(row_value(&row, 4, "snapshot.candidate_id")?)?,
        chat_guid: row_value(&row, 5, "snapshot.chat_guid")?.to_owned(),
        anchor_message_guid: row_value(&row, 6, "snapshot.anchor_message_guid")?.to_owned(),
        diagnostics: DiagnosticsTraceLinkage {
            trace_id: optional_cell(row_value(&row, 7, "snapshot.diagnostics_trace_id")?),
            span_id: optional_cell(row_value(&row, 8, "snapshot.diagnostics_span_id")?),
            parent_span_id: optional_cell(row_value(
                &row,
                9,
                "snapshot.diagnostics_parent_span_id",
            )?),
            chat_hash: optional_cell(row_value(&row, 10, "snapshot.diagnostics_chat_hash")?),
            message_hash: optional_cell(row_value(&row, 11, "snapshot.diagnostics_message_hash")?),
        },
        provider_model_prompt_version_id: optional_i64_cell(
            row_value(&row, 12, "snapshot.provider_model_prompt_version_id")?,
            "provider_model_prompt_version_id",
        )?,
        source_excerpt_policy: FeedbackSourceExcerptPolicy::parse(row_value(
            &row,
            13,
            "snapshot.source_excerpt_policy",
        )?)?,
        label_source: FeedbackLabelSource::parse(row_value(&row, 14, "snapshot.label_source")?)?,
        privacy_tier: FeedbackPrivacyTier::parse(row_value(&row, 15, "snapshot.privacy_tier")?)?,
        privacy_metadata_json: row_value(&row, 16, "snapshot.privacy_metadata_json")?.to_owned(),
        created_at: row_i64(&row, 25, "snapshot.created_at")?,
        expires_at: optional_i64_cell(
            row_value(&row, 26, "snapshot.expires_at")?,
            "snapshot.expires_at",
        )?,
    };
    super::feedback_eval_validation::validate_meta(&meta)?;
    let label_type = FeedbackLabelType::parse(row_value(&row, 28, "label.label_type")?)?;
    let label_value = FeedbackLabelValue::parse(
        label_type,
        row_value(&row, 29, "label.label_value")?,
        "label_value",
    )?;
    Ok(EvalCase {
        snapshot: FeatureSnapshot {
            id: None,
            snapshot_key: row_value(&row, 0, "snapshot.snapshot_key")?.to_owned(),
            meta: meta.clone(),
            route: optional_cell(row_value(&row, 17, "snapshot.route")?),
            reason_code: optional_cell(row_value(&row, 18, "snapshot.reason_code")?),
            confidence_millis: optional_i64_cell(
                row_value(&row, 19, "snapshot.confidence_millis")?,
                "confidence_millis",
            )?,
            participant_count: optional_i64_cell(
                row_value(&row, 20, "snapshot.participant_count")?,
                "participant_count",
            )?,
            tapback_signal: optional_cell(row_value(&row, 21, "snapshot.tapback_signal")?),
            sender_signal_available: row_bool(&row, 22, "sender_signal_available")?,
            context_window_available: row_bool(&row, 23, "context_window_available")?,
            excerpt: optional_cell(row_value(&row, 24, "snapshot.excerpt")?),
        },
        label: Label {
            id: None,
            label_key: row_value(&row, 27, "label.label_key")?.to_owned(),
            label_value,
            meta,
        },
    })
}

fn row_i64(row: &[String], index: usize, field: &'static str) -> Result<i64, StorageError> {
    row_value(row, index, field)?
        .parse::<i64>()
        .map_err(|err| StorageError::Sqlite {
            message: format!("expected integer {field}: {err}"),
        })
}

fn row_bool(row: &[String], index: usize, field: &'static str) -> Result<bool, StorageError> {
    match row_value(row, index, field)? {
        "0" => Ok(false),
        "1" => Ok(true),
        other => Err(StorageError::Sqlite {
            message: format!("expected boolean {field}, got {other}"),
        }),
    }
}

fn optional_cell(value: &str) -> Option<String> {
    match value {
        "" => None,
        value => Some(value.to_owned()),
    }
}

fn optional_candidate(value: &str) -> Result<Option<CandidateId>, StorageError> {
    match optional_cell(value) {
        Some(value) => CandidateId::from_storage(&value).map(Some),
        None => Ok(None),
    }
}

fn optional_i64_cell(value: &str, field: &'static str) -> Result<Option<i64>, StorageError> {
    match optional_cell(value) {
        Some(value) => value
            .parse::<i64>()
            .map(Some)
            .map_err(|err| StorageError::Sqlite {
                message: format!("expected integer {field}: {err}"),
            }),
        None => Ok(None),
    }
}
