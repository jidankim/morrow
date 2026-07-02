use morrow_storage::{
    CandidateDraft, CandidateKind, DiagnosticsTraceLinkage, FeatureSnapshot, FeedbackLabelSource,
    FeedbackLabelValue, FeedbackPrivacyTier, FeedbackRecordMeta, FeedbackSourceExcerptPolicy,
    FeedbackSubjectType, Label, Store,
};

pub fn fresh_store(name: &str) -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join(name);
    let store = Store::open(&db_path).expect("open store");
    (dir, store)
}

pub fn meta(subject_type: FeedbackSubjectType, subject_id: &str) -> FeedbackRecordMeta {
    FeedbackRecordMeta {
        schema_version: 1,
        subject_type,
        subject_id: subject_id.to_owned(),
        candidate_id: None,
        chat_guid: "chat-guid-private".to_owned(),
        anchor_message_guid: "message-guid-private".to_owned(),
        diagnostics: trace_linkage(),
        provider_model_prompt_version_id: None,
        source_excerpt_policy: FeedbackSourceExcerptPolicy::Hide,
        label_source: FeedbackLabelSource::Provider,
        privacy_tier: FeedbackPrivacyTier::InternalMetadata,
        privacy_metadata_json: "{}".to_owned(),
        created_at: 1_783_000_000,
        expires_at: Some(1_785_592_000),
    }
}

pub fn label(
    label_key: &str,
    subject_type: FeedbackSubjectType,
    subject_id: &str,
    value: FeedbackLabelValue,
) -> Label {
    Label {
        id: None,
        label_key: label_key.to_owned(),
        label_value: value,
        meta: meta(subject_type, subject_id),
    }
}

pub fn snapshot(
    snapshot_key: &str,
    subject_type: FeedbackSubjectType,
    subject_id: &str,
) -> FeatureSnapshot {
    FeatureSnapshot {
        id: None,
        snapshot_key: snapshot_key.to_owned(),
        meta: meta(subject_type, subject_id),
        route: Some("provider_candidate".to_owned()),
        reason_code: Some("provider_valid".to_owned()),
        confidence_millis: Some(860),
        participant_count: Some(2),
        tapback_signal: Some("absent".to_owned()),
        sender_signal_available: false,
        context_window_available: false,
        excerpt: Some("Source excerpt hidden by settings.".to_owned()),
    }
}

pub fn candidate_draft() -> CandidateDraft {
    CandidateDraft {
        kind: CandidateKind::CalendarEvent,
        chat_guid: "chat-guid-private".to_owned(),
        anchor_message_guid: "message-guid-private".to_owned(),
        title: "private title".to_owned(),
        confidence_millis: 860,
        normalized_time: "2026-07-15T00:00:00Z".to_owned(),
        evidence_excerpt: "private message excerpt".to_owned(),
        observed_at: 1_783_000_000,
    }
}

fn trace_linkage() -> DiagnosticsTraceLinkage {
    DiagnosticsTraceLinkage {
        trace_id: Some("trace_018fda8a98bf4cdba33a6f9d42180d6d".to_owned()),
        span_id: Some("span_52efdebab8574a5fa6f290831a786e86".to_owned()),
        parent_span_id: Some("span_62efdebab8574a5fa6f290831a786e86".to_owned()),
        chat_hash: Some(
            "sha256:66e0bc3220b7dd3d0651965d244ebba7f5a8ae571be6874570b58495cdf26d85".to_owned(),
        ),
        message_hash: Some(
            "sha256:d9cee5362324ef2404962c149f0c564c7f6f009fe91ba5985cc5341a79a348de".to_owned(),
        ),
    }
}
