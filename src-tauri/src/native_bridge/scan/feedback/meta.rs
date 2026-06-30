use morrow_diagnostics::{TraceRecord, TraceSchemaVersion};
use morrow_storage::{
    CandidateId, DiagnosticsTraceLinkage, FeedbackLabelSource, FeedbackPrivacyTier,
    FeedbackRecordMeta, FeedbackSourceExcerptPolicy, FeedbackSubjectType,
    FEEDBACK_EVAL_SCHEMA_VERSION,
};
use serde_json::json;

use crate::native_bridge::scan::config::ScanConfig;
use crate::native_bridge::scan::ScanSelectedChatsError;

const SNAPSHOT_TTL_SECONDS: i64 = 30 * 24 * 60 * 60;
const REDACTED_SOURCE_EXCERPT: &str = "Source excerpt hidden by settings.";

pub(super) struct CandidateSubject<'a> {
    pub(super) chat_guid: &'a str,
    pub(super) anchor_message_guid: &'a str,
    pub(super) created_at: i64,
}

pub(super) struct MetaInput<'a> {
    pub(super) subject_type: FeedbackSubjectType,
    pub(super) subject_id: &'a str,
    pub(super) candidate_id: Option<CandidateId>,
    pub(super) subject: CandidateSubject<'a>,
    pub(super) label_source: FeedbackLabelSource,
    pub(super) trace: Option<&'a TraceRecord>,
    pub(super) config: &'a ScanConfig,
}

pub(super) fn feedback_meta(
    input: MetaInput<'_>,
) -> Result<FeedbackRecordMeta, ScanSelectedChatsError> {
    Ok(FeedbackRecordMeta {
        schema_version: FEEDBACK_EVAL_SCHEMA_VERSION,
        subject_type: input.subject_type,
        subject_id: input.subject_id.to_owned(),
        candidate_id: input.candidate_id,
        chat_guid: input.subject.chat_guid.to_owned(),
        anchor_message_guid: input.subject.anchor_message_guid.to_owned(),
        diagnostics: diagnostics(input.trace),
        provider_model_prompt_version_id: None,
        source_excerpt_policy: source_excerpt_policy(input.config),
        label_source: input.label_source,
        privacy_tier: privacy_tier(input.config),
        privacy_metadata_json: privacy_metadata(input.trace, input.config)?,
        created_at: input.subject.created_at,
        expires_at: Some(input.subject.created_at + SNAPSHOT_TTL_SECONDS),
    })
}

pub(super) fn snapshot_excerpt(excerpt: &str, config: &ScanConfig) -> String {
    if config.feedback_text_snapshots_enabled {
        excerpt.to_owned()
    } else {
        REDACTED_SOURCE_EXCERPT.to_owned()
    }
}

pub(super) fn tapback_signal(value: bool) -> &'static str {
    match value {
        true => "present",
        false => "absent",
    }
}

fn privacy_metadata(
    trace: Option<&TraceRecord>,
    config: &ScanConfig,
) -> Result<String, ScanSelectedChatsError> {
    serde_json::to_string(&json!({
        "snapshot_schema_version": FEEDBACK_EVAL_SCHEMA_VERSION,
        "trace_schema_version": trace.map(trace_schema_version),
        "feedback_text_snapshots_enabled": config.feedback_text_snapshots_enabled,
        "provider_id": trace.and_then(|record| record.span.provider_id.clone()),
        "model_id": trace.and_then(|record| record.span.model_id.clone()),
        "template_version": trace.and_then(|record| record.span.template_version.clone()),
        "sender_signal_available": false,
        "context_window_available": false
    }))
    .map_err(|error| ScanSelectedChatsError::Detection(error.to_string()))
}

fn diagnostics(trace: Option<&TraceRecord>) -> DiagnosticsTraceLinkage {
    match trace {
        Some(record) => DiagnosticsTraceLinkage {
            trace_id: Some(record.trace.trace_id.clone()),
            span_id: Some(record.trace.span_id.clone()),
            parent_span_id: record.trace.parent_span_id.clone(),
            chat_hash: record.trace.chat_hash.clone(),
            message_hash: record.trace.message_hash.clone(),
        },
        None => DiagnosticsTraceLinkage {
            trace_id: None,
            span_id: None,
            parent_span_id: None,
            chat_hash: None,
            message_hash: None,
        },
    }
}

fn trace_schema_version(record: &TraceRecord) -> &'static str {
    match record.schema_version {
        TraceSchemaVersion::V1 => "v1",
    }
}

fn source_excerpt_policy(config: &ScanConfig) -> FeedbackSourceExcerptPolicy {
    match config.detection.source_excerpts {
        morrow_detection::SourceExcerptPolicy::Include => FeedbackSourceExcerptPolicy::Include,
        morrow_detection::SourceExcerptPolicy::Hide => FeedbackSourceExcerptPolicy::Hide,
    }
}

fn privacy_tier(config: &ScanConfig) -> FeedbackPrivacyTier {
    if config.feedback_text_snapshots_enabled {
        FeedbackPrivacyTier::LocalPrivate
    } else {
        FeedbackPrivacyTier::InternalMetadata
    }
}
