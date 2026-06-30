use morrow_storage::{
    delete_all_at, DeleteAllConfirmation, DiagnosticsTraceLinkage, EvalResult, EvalResultOutcome,
    EvalRun, EvalRunStatus, FeatureSnapshot, FeedbackEvent, FeedbackEventType, FeedbackLabelSource,
    FeedbackLabelType, FeedbackLabelValue, FeedbackPrivacyTier, FeedbackRecordMeta,
    FeedbackSourceExcerptPolicy, FeedbackSubjectType, Label, ProposalOutcomeLabel, StorageError,
    Store,
};

fn fresh_store(name: &str) -> (tempfile::TempDir, std::path::PathBuf, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join(name);
    let store = Store::open(&db_path).expect("open store");
    (dir, db_path, store)
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
            parent_span_id: Some("span_62efdebab8574a5fa6f290831a786e86".to_owned()),
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

fn eval_run(run_key: &str) -> EvalRun {
    EvalRun {
        id: None,
        run_key: run_key.to_owned(),
        status: EvalRunStatus::Passed,
        schema_version: 1,
        started_at: 1_783_000_100,
        finished_at: Some(1_783_000_101),
        report_path: Some("/tmp/morrow-feedback-eval-report.json".to_owned()),
        cases_evaluated: 1,
        cases_skipped: 0,
        skip_reasons_json: "{}".to_owned(),
        approval_kept_rate_millis: 1000,
        observed_rejection_rate_millis: 0,
        quiet_log_count: 0,
        provider_failure_count: 0,
        accepted_visible_ratio_millis: 1000,
        confusion_counts_json: "{}".to_owned(),
    }
}

fn eval_result(result_key: &str, eval_run_id: i64) -> EvalResult {
    EvalResult {
        id: None,
        result_key: result_key.to_owned(),
        eval_run_id,
        snapshot_key: "snapshot-key".to_owned(),
        label_key: "label-key".to_owned(),
        expected_label_type: FeedbackLabelType::ProposalOutcome,
        expected_label_value: "accepted".to_owned(),
        actual_label_type: Some(FeedbackLabelType::DetectionRoute),
        actual_label_value: Some("deterministic_candidate".to_owned()),
        outcome: EvalResultOutcome::Match,
        skip_reason: None,
        created_at: 1_783_000_102,
    }
}

#[test]
fn feedback_eval_records_validate_privacy_summary_counts_excerpt_bytes() {
    // Given
    let (_dir, _db_path, store) = fresh_store("feedback-eval-records.sqlite");

    // When
    store
        .record_feedback_event(feedback_event("event-key"))
        .expect("record feedback event");
    store
        .record_label(label("label-key"))
        .expect("record label");
    store
        .record_feature_snapshot(feature_snapshot("snapshot-key"))
        .expect("record feature snapshot");
    let eval_run_id = store
        .create_eval_run(eval_run("run-key"))
        .expect("eval run");
    store
        .record_eval_result(eval_result("result-key", eval_run_id))
        .expect("record eval result");

    store
        .record_feedback_event(feedback_event("event-key"))
        .expect("duplicate event is idempotent");
    store
        .record_label(label("label-key"))
        .expect("duplicate label is idempotent");
    store
        .record_feature_snapshot(feature_snapshot("snapshot-key"))
        .expect("duplicate snapshot is idempotent");
    let duplicate_run_id = store
        .create_eval_run(eval_run("run-key"))
        .expect("duplicate run is idempotent");
    store
        .record_eval_result(eval_result("result-key", duplicate_run_id))
        .expect("duplicate result is idempotent");

    let cases = store.eval_cases().expect("eval cases");
    let summary = store.privacy_summary().expect("privacy summary");

    // Then
    assert_eq!(eval_run_id, duplicate_run_id);
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].snapshot.snapshot_key, "snapshot-key");
    assert_eq!(
        cases[0].label.label_value,
        FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::Accepted)
    );
    assert_eq!(summary.full_message_body_columns, 0);
    assert_eq!(summary.max_excerpt_len, "meet on July 15".len());
}

#[test]
fn feedback_eval_delete_all_removes_new_data() {
    // Given
    let (_dir, db_path, store) = fresh_store("feedback-eval-delete-all.sqlite");
    store
        .record_feedback_event(feedback_event("delete-event"))
        .expect("record feedback event");
    store
        .record_label(label("delete-label"))
        .expect("record label");
    store
        .record_feature_snapshot(feature_snapshot("delete-snapshot"))
        .expect("record feature snapshot");
    let invalid_confirmation = DeleteAllConfirmation::parse("delete morrow data");

    // When
    assert!(invalid_confirmation.is_err());
    assert!(db_path.exists());
    assert_eq!(
        Store::open(&db_path)
            .expect("reopen before delete")
            .eval_cases()
            .expect("eval cases before delete")
            .len(),
        1
    );
    let confirmation = DeleteAllConfirmation::parse("DELETE MORROW DATA").expect("confirmation");
    let receipt = delete_all_at(&db_path, confirmation).expect("delete all");

    // Then
    assert!(receipt.database_deleted);
    assert!(!db_path.exists());
}

#[test]
fn feedback_eval_invalid_label_rejected() {
    // Given
    let (_dir, _db_path, store) = fresh_store("feedback-eval-invalid-label.sqlite");
    store
        .record_label(label("label-key"))
        .expect("record label");
    store
        .record_feature_snapshot(feature_snapshot("snapshot-key"))
        .expect("record feature snapshot");
    let eval_run_id = store
        .create_eval_run(eval_run("run-key"))
        .expect("eval run");
    let mut result = eval_result("result-key", eval_run_id);
    result.expected_label_value = "not_allowed".to_owned();

    // When
    let error = store.record_eval_result(result).expect_err("invalid label");

    // Then
    match error {
        StorageError::InvalidInput { field, .. } => {
            assert_eq!(field, "expected_label_value");
        }
        other => panic!("expected invalid input, got {other}"),
    }
}

#[test]
fn feedback_eval_rejects_malformed_boundaries() {
    // Given
    let (_dir, _db_path, store) = fresh_store("feedback-eval-malformed.sqlite");

    let mut long_event = feedback_event("event-key");
    long_event.meta.subject_id = "x".repeat(241);
    let mut bad_schema = label("label-key");
    bad_schema.meta.schema_version = 2;
    let mut bad_expiry = label("expiry-label");
    bad_expiry.meta.expires_at = Some(bad_expiry.meta.created_at);
    let mut bad_diagnostics = label("diagnostics-label");
    bad_diagnostics.meta.diagnostics.trace_id = Some("chat-guid-raw".to_owned());
    let mut hidden_text = feature_snapshot("hidden-snapshot");
    hidden_text.meta.source_excerpt_policy = FeedbackSourceExcerptPolicy::Hide;
    hidden_text.excerpt = Some("meet on July 15".to_owned());

    // When / Then
    assert_invalid_input(store.record_feedback_event(long_event), "subject_id");
    assert_invalid_input(store.record_label(bad_schema), "schema_version");
    assert_invalid_input(store.record_label(bad_expiry), "expires_at");
    assert_invalid_input(store.record_label(bad_diagnostics), "diagnostics_trace_id");
    assert!(matches!(
        store.record_feature_snapshot(hidden_text),
        Err(StorageError::PrivacyViolation { .. })
    ));
}

fn assert_invalid_input<T>(result: Result<T, StorageError>, expected_field: &'static str) {
    match result {
        Err(StorageError::InvalidInput { field, .. }) => assert_eq!(field, expected_field),
        Ok(_) => panic!("expected invalid input for {expected_field}"),
        Err(other) => panic!("expected invalid input for {expected_field}, got {other}"),
    }
}
