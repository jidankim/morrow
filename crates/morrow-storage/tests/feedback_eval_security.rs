use morrow_storage::{
    DiagnosticsTraceLinkage, FeatureSnapshot, FeedbackEvent, FeedbackEventType,
    FeedbackLabelSource, FeedbackLabelValue, FeedbackPrivacyTier, FeedbackRecordMeta,
    FeedbackSourceExcerptPolicy, FeedbackSubjectType, Label, ProposalOutcomeLabel, StorageError,
    Store,
};

fn fresh_store(name: &str) -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join(name);
    let store = Store::open(&db_path).expect("open store");
    (dir, store)
}

fn meta(subject_id: &str) -> FeedbackRecordMeta {
    FeedbackRecordMeta {
        schema_version: 1,
        subject_type: FeedbackSubjectType::Candidate,
        subject_id: subject_id.to_owned(),
        candidate_id: None,
        chat_guid: "chat-feedback".to_owned(),
        anchor_message_guid: "message-feedback".to_owned(),
        diagnostics: DiagnosticsTraceLinkage {
            trace_id: Some("trace_018fda8a98bf4cdba33a6f9d42180d6d".to_owned()),
            span_id: Some("span_52efdebab8574a5fa6f290831a786e86".to_owned()),
            parent_span_id: None,
            chat_hash: Some(
                "sha256:66e0bc3220b7dd3d0651965d244ebba7f5a8ae571be6874570b58495cdf26d85"
                    .to_owned(),
            ),
            message_hash: Some(
                "sha256:d9cee5362324ef2404962c149f0c564c7f6f009fe91ba5985cc5341a79a348de"
                    .to_owned(),
            ),
        },
        provider_model_prompt_version_id: None,
        source_excerpt_policy: FeedbackSourceExcerptPolicy::Include,
        label_source: FeedbackLabelSource::Lifecycle,
        privacy_tier: FeedbackPrivacyTier::LocalPrivate,
        privacy_metadata_json: "{}".to_owned(),
        created_at: 1_783_000_000,
        expires_at: Some(1_783_086_400),
    }
}

fn feedback_event(event_key: &str) -> FeedbackEvent {
    FeedbackEvent {
        id: None,
        event_key: event_key.to_owned(),
        event_type: FeedbackEventType::CandidateVisible,
        meta: meta("subject-event"),
    }
}

fn label(label_key: &str) -> Label {
    Label {
        id: None,
        label_key: label_key.to_owned(),
        label_value: FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::Accepted),
        meta: meta("subject-snapshot"),
    }
}

fn feature_snapshot(snapshot_key: &str) -> FeatureSnapshot {
    FeatureSnapshot {
        id: None,
        snapshot_key: snapshot_key.to_owned(),
        meta: meta("subject-snapshot"),
        route: Some("deterministic_candidate".to_owned()),
        reason_code: Some("parser_match".to_owned()),
        confidence_millis: Some(930),
        participant_count: Some(3),
        tapback_signal: Some("liked".to_owned()),
        sender_signal_available: true,
        context_window_available: false,
        excerpt: Some("meet on July 15".to_owned()),
    }
}

#[test]
fn feedback_eval_records_use_bound_parameters_for_runtime_sql() {
    // Given
    let writer = include_str!("../src/store/feedback_eval_records.rs");

    // When / Then
    assert!(
        !writer.contains("sql_text"),
        "feedback/eval writes must use bound sqlite parameters, not sql_text escaping"
    );
}

#[test]
fn feedback_eval_records_preserve_sql_punctuation_as_data() {
    // Given
    let (_dir, store) = fresh_store("feedback-eval-sql-punctuation.sqlite");
    let adversarial = "subject'; DELETE FROM labels; --\n{\"json\":[\"quote's\",1]}";
    let mut event = feedback_event("event-sql-punctuation");
    event.meta.subject_id = adversarial.to_owned();
    event.meta.privacy_metadata_json = "{\"fixture\":\"quote';--\\n{}[]\"}".to_owned();
    let mut label = label("label-sql-punctuation");
    label.meta.subject_id = adversarial.to_owned();
    label.meta.privacy_metadata_json = event.meta.privacy_metadata_json.clone();
    let mut snapshot = feature_snapshot("snapshot-sql-punctuation");
    snapshot.meta.subject_id = adversarial.to_owned();
    snapshot.meta.privacy_metadata_json = event.meta.privacy_metadata_json.clone();
    snapshot.reason_code = Some("reason'; DROP TABLE feature_snapshots; --".to_owned());

    // When
    store
        .record_feedback_event(event)
        .expect("record adversarial event");
    store.record_label(label).expect("record adversarial label");
    store
        .record_feature_snapshot(snapshot)
        .expect("record adversarial snapshot");
    let cases = store.eval_cases().expect("eval cases");

    // Then
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].snapshot.meta.subject_id, adversarial);
    assert_eq!(cases[0].label.meta.subject_id, adversarial);
    assert_eq!(
        cases[0].snapshot.reason_code.as_deref(),
        Some("reason'; DROP TABLE feature_snapshots; --")
    );
    assert_eq!(store.feedback_eval_counts().expect("counts").label_count, 1);
}

#[test]
fn feedback_eval_rejects_forbidden_privacy_metadata_for_every_record_api() {
    // Given
    let (_dir, store) = fresh_store("feedback-eval-privacy-metadata.sqlite");
    let forbidden_metadata = [
        r#"{"prompt":"write the full message"}"#,
        r#"{"response_text":"raw assistant output"}"#,
        r#"{"safe_key":"contains message_history content"}"#,
    ];

    for (index, metadata) in forbidden_metadata.iter().enumerate() {
        let mut event = feedback_event(&format!("event-forbidden-meta-{index}"));
        event.meta.privacy_metadata_json = (*metadata).to_owned();
        let mut label = label(&format!("label-forbidden-meta-{index}"));
        label.meta.privacy_metadata_json = (*metadata).to_owned();
        let mut snapshot = feature_snapshot(&format!("snapshot-forbidden-meta-{index}"));
        snapshot.meta.privacy_metadata_json = (*metadata).to_owned();

        // When / Then
        assert_privacy_violation(store.record_feedback_event(event));
        assert_privacy_violation(store.record_label(label));
        assert_privacy_violation(store.record_feature_snapshot(snapshot));
    }
}

#[test]
fn feedback_eval_rejects_stemmed_forbidden_privacy_metadata_values() {
    // Given
    let (_dir, store) = fresh_store("feedback-eval-stemmed-forbidden-values.sqlite");
    let forbidden_metadata = [
        r#"{"fixture":"fullMessage"}"#,
        r#"{"fixture":"full-message"}"#,
        r#"{"fixture":"full message"}"#,
        r#"{"fixture":"rawMessage"}"#,
        r#"{"fixture":"private message"}"#,
    ];

    for (index, metadata) in forbidden_metadata.iter().enumerate() {
        let mut event = feedback_event(&format!("event-stemmed-forbidden-value-{index}"));
        event.meta.privacy_metadata_json = (*metadata).to_owned();

        // When / Then
        assert_privacy_violation(store.record_feedback_event(event));
    }
}

#[test]
fn feedback_eval_reports_stemmed_forbidden_privacy_metadata_keys_as_private_content() {
    // Given
    let (_dir, store) = fresh_store("feedback-eval-stemmed-forbidden-keys.sqlite");
    let forbidden_metadata = [
        r#"{"fullMessage":true}"#,
        r#"{"full-message":true}"#,
        r#"{"full message":true}"#,
        r#"{"rawMessage":true}"#,
        r#"{"private message":true}"#,
    ];

    for (index, metadata) in forbidden_metadata.iter().enumerate() {
        let mut event = feedback_event(&format!("event-stemmed-forbidden-key-{index}"));
        event.meta.privacy_metadata_json = (*metadata).to_owned();

        // When / Then
        assert_privacy_violation_reason(
            store.record_feedback_event(event),
            "privacy metadata key names private message content",
        );
    }
}

#[test]
fn feedback_eval_rejects_unicode_escaped_forbidden_privacy_metadata_values() {
    // Given
    let (_dir, store) = fresh_store("feedback-eval-escaped-forbidden-values.sqlite");
    let mut event = feedback_event("event-escaped-forbidden-value");
    event.meta.privacy_metadata_json =
        r#"{"fixture":"\u0072\u0065\u0073\u0070\u006f\u006e\u0073\u0065"}"#.to_owned();

    // When / Then
    assert_privacy_violation(store.record_feedback_event(event));
}

#[test]
fn feedback_eval_rejects_unicode_escaped_forbidden_privacy_metadata_keys() {
    // Given
    let (_dir, store) = fresh_store("feedback-eval-escaped-forbidden-keys.sqlite");
    let mut event = feedback_event("event-escaped-forbidden-key");
    event.meta.privacy_metadata_json =
        r#"{"\u0070\u0072\u006f\u006d\u0070\u0074schema_version":1}"#.to_owned();

    // When / Then
    assert_privacy_violation(store.record_feedback_event(event));
}

fn assert_privacy_violation<T>(result: Result<T, StorageError>) {
    match result {
        Err(StorageError::PrivacyViolation { .. }) => {}
        Ok(_) => panic!("expected privacy violation"),
        Err(other) => panic!("expected privacy violation, got {other}"),
    }
}

fn assert_privacy_violation_reason<T>(result: Result<T, StorageError>, expected_reason: &str) {
    match result {
        Err(StorageError::PrivacyViolation { reason }) => assert_eq!(reason, expected_reason),
        Ok(_) => panic!("expected privacy violation"),
        Err(other) => panic!("expected privacy violation, got {other}"),
    }
}
