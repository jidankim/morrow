use morrow_storage::{
    delete_all_at, CandidateDraft, CandidateKind, CandidateState, DeleteAllConfirmation,
    DiagnosticsTraceLinkage, EvalRun, EvalRunStatus, ExternalObjectMapping, ExternalSource,
    FeatureSnapshot, FeedbackLabelSource, FeedbackLabelValue, FeedbackPrivacyTier,
    FeedbackRecordMeta, FeedbackSourceExcerptPolicy, FeedbackSubjectType, Label,
    ProposalOutcomeLabel, Store, FEEDBACK_EVAL_SCHEMA_VERSION,
};

#[test]
fn feedback_eval_invalid_delete_all_preserves_tables() {
    // Given
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("feedback-eval-invalid-delete-all.sqlite");
    let store = Store::open(&db_path).expect("open store");
    store
        .record_feature_snapshot(feature_snapshot())
        .expect("record feature snapshot");
    store.record_label(label()).expect("record label");
    store.create_eval_run(eval_run()).expect("create eval run");

    // When
    let invalid_confirmation = DeleteAllConfirmation::parse("delete morrow data");
    let reopened = Store::open(&db_path).expect("reopen store");

    // Then
    assert!(invalid_confirmation.is_err());
    assert!(db_path.exists());
    assert_eq!(reopened.eval_cases().expect("eval cases").len(), 1);
    let counts = reopened
        .feedback_eval_counts()
        .expect("feedback eval counts");
    assert_eq!(counts.label_count, 1);
    assert_eq!(counts.feature_snapshot_count, 1);
    assert_eq!(counts.latest_eval_status, Some(EvalRunStatus::Passed));
}

#[test]
fn delete_all_preserves_apple_and_approved_external_fixtures_while_removing_feedback_db() {
    // Given
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("morrow.sqlite");
    let apple_messages_db = dir.path().join("Apple Messages").join("chat.db");
    let approved_calendar_item = dir.path().join("Apple Calendar").join("approved-event.ics");
    let approved_reminder_item = dir
        .path()
        .join("Apple Reminders")
        .join("approved-reminder.json");
    let unrelated_diagnostics_sibling = dir.path().join("diagnostics").join("manual-notes");
    let removable_trace_artifact = dir.path().join("diagnostics").join("traces");

    std::fs::create_dir_all(apple_messages_db.parent().expect("messages parent"))
        .expect("create messages dir");
    std::fs::create_dir_all(approved_calendar_item.parent().expect("calendar parent"))
        .expect("create calendar dir");
    std::fs::create_dir_all(approved_reminder_item.parent().expect("reminders parent"))
        .expect("create reminders dir");
    std::fs::create_dir_all(&unrelated_diagnostics_sibling).expect("create diagnostics sibling");
    std::fs::create_dir_all(&removable_trace_artifact).expect("create trace artifact");
    std::fs::write(&apple_messages_db, b"fake apple messages").expect("write messages fixture");
    std::fs::write(
        &approved_calendar_item,
        b"BEGIN:VEVENT\nSTATUS:CONFIRMED\nEND:VEVENT",
    )
    .expect("write approved calendar fixture");
    std::fs::write(&approved_reminder_item, b"{\"status\":\"approved\"}")
        .expect("write approved reminder fixture");
    std::fs::write(
        unrelated_diagnostics_sibling.join("keep.txt"),
        b"manual diagnostic note",
    )
    .expect("write diagnostics sibling fixture");
    std::fs::write(removable_trace_artifact.join("trace.jsonl"), b"{}")
        .expect("write trace artifact");

    let store = Store::open(&db_path).expect("open store");
    store
        .record_feature_snapshot(feature_snapshot())
        .expect("record feature snapshot");
    store.record_label(label()).expect("record label");
    store.create_eval_run(eval_run()).expect("create eval run");
    let candidate_id = store
        .create_candidate(approved_calendar_draft())
        .expect("create approved candidate");
    store
        .transition_candidate(
            &candidate_id,
            CandidateState::CreatingExternal,
            "calendar proposal started",
            1_783_000_002,
        )
        .expect("transition creating");
    store
        .transition_candidate(
            &candidate_id,
            CandidateState::Visible,
            "proposal visible",
            1_783_000_003,
        )
        .expect("transition visible");
    store
        .record_candidate_external_receipt(&ExternalObjectMapping {
            candidate_id: candidate_id.clone(),
            source: ExternalSource::Calendar,
            external_object_id: "eventkit://calendar/approved/fixture".to_owned(),
            external_source_id: "calendar-source-approved".to_owned(),
            mapped_at: 1_783_000_004,
        })
        .expect("record external receipt");
    store
        .transition_candidate(
            &candidate_id,
            CandidateState::Approved,
            "user approved in Apple Calendar",
            1_783_000_005,
        )
        .expect("transition approved");

    let counts = store
        .feedback_eval_counts()
        .expect("feedback eval counts before delete");
    let mapping = store
        .candidate_external_mapping(&candidate_id, ExternalSource::Calendar, 1_783_000_004)
        .expect("external mapping before delete")
        .expect("external mapping exists before delete");
    let confirmation = DeleteAllConfirmation::parse("DELETE MORROW DATA").expect("confirmation");
    drop(store);

    // When
    let receipt = delete_all_at(&db_path, confirmation).expect("delete all");

    // Then
    assert_eq!(counts.label_count, 1);
    assert_eq!(counts.feature_snapshot_count, 1);
    assert_eq!(
        mapping.external_object_id,
        "eventkit://calendar/approved/fixture"
    );
    assert!(receipt.database_deleted);
    assert!(receipt.diagnostics_artifacts_deleted);
    assert!(!receipt.approved_external_items_deleted);
    assert!(!db_path.exists());
    assert!(!removable_trace_artifact.exists());
    assert!(apple_messages_db.exists());
    assert!(approved_calendar_item.exists());
    assert!(approved_reminder_item.exists());
    assert!(unrelated_diagnostics_sibling.join("keep.txt").exists());
}

fn feature_snapshot() -> FeatureSnapshot {
    FeatureSnapshot {
        id: None,
        snapshot_key: "invalid-delete-all-snapshot".to_owned(),
        meta: meta(),
        route: Some("deterministic_candidate".to_owned()),
        reason_code: Some("invalid_delete_all_fixture".to_owned()),
        confidence_millis: Some(900),
        participant_count: Some(2),
        tapback_signal: Some("fixture".to_owned()),
        sender_signal_available: false,
        context_window_available: false,
        excerpt: Some("Source excerpt hidden by settings.".to_owned()),
    }
}

fn label() -> Label {
    Label {
        id: None,
        label_key: "invalid-delete-all-label".to_owned(),
        label_value: FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::Accepted),
        meta: meta(),
    }
}

fn eval_run() -> EvalRun {
    EvalRun {
        id: None,
        run_key: "invalid-delete-all-run".to_owned(),
        status: EvalRunStatus::Passed,
        schema_version: FEEDBACK_EVAL_SCHEMA_VERSION,
        started_at: 1_783_000_000,
        finished_at: Some(1_783_000_001),
        report_path: Some("/tmp/morrow-invalid-delete-all-report.json".to_owned()),
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

fn approved_calendar_draft() -> CandidateDraft {
    CandidateDraft {
        kind: CandidateKind::CalendarEvent,
        chat_guid: "chat-approved-delete-all".to_owned(),
        anchor_message_guid: "message-approved-delete-all".to_owned(),
        title: "Approved fixture".to_owned(),
        confidence_millis: 910,
        normalized_time: "2026-07-15T19:00:00Z".to_owned(),
        evidence_excerpt: "approved event fixture".to_owned(),
        observed_at: 1_783_000_001,
    }
}

fn meta() -> FeedbackRecordMeta {
    FeedbackRecordMeta {
        schema_version: FEEDBACK_EVAL_SCHEMA_VERSION,
        subject_type: FeedbackSubjectType::EvalCase,
        subject_id: "invalid-delete-all-subject".to_owned(),
        candidate_id: None,
        chat_guid: "chat-invalid-delete-all".to_owned(),
        anchor_message_guid: "message-invalid-delete-all".to_owned(),
        diagnostics: DiagnosticsTraceLinkage {
            trace_id: None,
            span_id: None,
            parent_span_id: None,
            chat_hash: None,
            message_hash: None,
        },
        provider_model_prompt_version_id: None,
        source_excerpt_policy: FeedbackSourceExcerptPolicy::Hide,
        label_source: FeedbackLabelSource::ManualAlpha,
        privacy_tier: FeedbackPrivacyTier::LocalPrivate,
        privacy_metadata_json: "{}".to_owned(),
        created_at: 1_783_000_000,
        expires_at: None,
    }
}
