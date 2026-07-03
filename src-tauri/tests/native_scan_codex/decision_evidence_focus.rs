use morrow_lib::native_bridge::{
    load_decision_evidence_at, CodexAuthStatus, LoadDecisionEvidenceRequest,
    NativeDecisionEvidenceSubjectType,
};
use morrow_storage::{
    CandidateDraft, CandidateKind, FeatureSnapshot, FeedbackLabelSource, FeedbackLabelValue,
    FeedbackPrivacyTier, FeedbackRecordMeta, FeedbackSourceExcerptPolicy, FeedbackSubjectType,
    Label, QuietLogDraft, Store, SystemOutcomeLabel, FEEDBACK_EVAL_SCHEMA_VERSION,
};

use super::support::{
    auth_readiness, candidate_json, scan_request_at, FakeCodexOutcome, RecordingCodexRunner,
    RejectingProposalAdapter, ScanFixture,
};

#[test]
fn decision_evidence_focuses_created_candidate_ids_over_recent_unrelated_evidence(
) -> Result<(), String> {
    // Given
    let fixture = ScanFixture::new("codex-decision-evidence-created-id-focus")?;
    let runner = RecordingCodexRunner::new(vec![FakeCodexOutcome::WriteOutput(candidate_json())]);
    let adapter = RejectingProposalAdapter;
    let scan = fixture.scan_with_request_and_app_data_dir(
        scan_request_at(1_782_352_400)?,
        auth_readiness(CodexAuthStatus::LoggedInUsingChatGpt, true),
        &runner,
        &adapter,
    )?;
    let created_candidate_id = scan
        .created_candidate_ids
        .first()
        .cloned()
        .ok_or_else(|| format!("scan did not return created candidate ids: {scan:#?}"))?;
    record_recent_unrelated_quiet_evidence(fixture.store_path())?;

    // When
    let recent_report = load_decision_evidence_at(
        fixture.store_path(),
        fixture.app_data_dir(),
        LoadDecisionEvidenceRequest {
            limit: 1,
            created_candidate_ids: Vec::new(),
        },
    )?;
    let focused_report = load_decision_evidence_at(
        fixture.store_path(),
        fixture.app_data_dir(),
        LoadDecisionEvidenceRequest {
            limit: 10,
            created_candidate_ids: vec![created_candidate_id.clone()],
        },
    )?;

    // Then
    assert_eq!(recent_report.items.len(), 1, "{recent_report:#?}");
    assert_eq!(
        recent_report.items[0].subject_type,
        NativeDecisionEvidenceSubjectType::QuietLog
    );
    assert_eq!(focused_report.items.len(), 1, "{focused_report:#?}");
    let focused = &focused_report.items[0];
    assert_eq!(
        focused.subject_type,
        NativeDecisionEvidenceSubjectType::Candidate
    );
    assert_eq!(
        focused.candidate_id.as_deref(),
        Some(created_candidate_id.as_str())
    );
    assert_eq!(focused.route.as_deref(), Some("provider_candidate"));
    Ok(())
}

#[test]
fn decision_evidence_filters_all_created_candidate_ids_before_result_limit() -> Result<(), String> {
    // Given
    let fixture = ScanFixture::new("codex-decision-evidence-created-id-limit")?;
    let runner = RecordingCodexRunner::new(vec![FakeCodexOutcome::WriteOutput(candidate_json())]);
    let adapter = RejectingProposalAdapter;
    let scan = fixture.scan_with_request_and_app_data_dir(
        scan_request_at(1_782_352_400)?,
        auth_readiness(CodexAuthStatus::LoggedInUsingChatGpt, true),
        &runner,
        &adapter,
    )?;
    let created_candidate_id = scan
        .created_candidate_ids
        .first()
        .cloned()
        .ok_or_else(|| format!("scan did not return created candidate ids: {scan:#?}"))?;
    let unused_candidate_id = Store::open(fixture.store_path())
        .map_err(|error| error.to_string())?
        .create_candidate(native_candidate_draft("native-limit-without-evidence"))
        .map_err(|error| error.to_string())?;

    // When
    let focused_report = load_decision_evidence_at(
        fixture.store_path(),
        fixture.app_data_dir(),
        LoadDecisionEvidenceRequest {
            limit: 1,
            created_candidate_ids: vec![
                unused_candidate_id.as_str().to_owned(),
                created_candidate_id.clone(),
            ],
        },
    )?;

    // Then
    assert_eq!(focused_report.items.len(), 1, "{focused_report:#?}");
    let focused = &focused_report.items[0];
    assert_eq!(
        focused.subject_type,
        NativeDecisionEvidenceSubjectType::Candidate
    );
    assert_eq!(
        focused.candidate_id.as_deref(),
        Some(created_candidate_id.as_str())
    );
    assert_eq!(focused.route.as_deref(), Some("provider_candidate"));
    Ok(())
}

fn record_recent_unrelated_quiet_evidence(store_path: &std::path::Path) -> Result<(), String> {
    let store = Store::open(store_path).map_err(|error| error.to_string())?;
    let subject_id = "quiet-recent-unrelated";
    store
        .record_quiet_log(QuietLogDraft {
            chat_guid: "chat-guid-private".to_owned(),
            anchor_message_guid: "message-guid-private".to_owned(),
            reason: "provider_unavailable".to_owned(),
            excerpt: "Source excerpt hidden by settings.".to_owned(),
            created_at: 1_782_352_900,
            provider_diagnostic: None,
        })
        .map_err(|error| error.to_string())?;
    let meta = recent_quiet_meta(subject_id);
    store
        .record_label(Label {
            id: None,
            label_key: "quiet-recent-unrelated-label".to_owned(),
            label_value: FeedbackLabelValue::SystemOutcome(SystemOutcomeLabel::FailedProvider),
            meta: meta.clone(),
        })
        .map_err(|error| error.to_string())?;
    store
        .record_feature_snapshot(FeatureSnapshot {
            id: None,
            snapshot_key: "quiet-recent-unrelated-snapshot".to_owned(),
            meta,
            route: Some("provider_unavailable".to_owned()),
            reason_code: Some("provider_timeout".to_owned()),
            confidence_millis: None,
            participant_count: Some(1),
            tapback_signal: Some("absent".to_owned()),
            sender_signal_available: false,
            context_window_available: false,
            excerpt: Some("Source excerpt hidden by settings.".to_owned()),
        })
        .map_err(|error| error.to_string())
}

fn recent_quiet_meta(subject_id: &str) -> FeedbackRecordMeta {
    FeedbackRecordMeta {
        schema_version: FEEDBACK_EVAL_SCHEMA_VERSION,
        subject_type: FeedbackSubjectType::QuietLog,
        subject_id: subject_id.to_owned(),
        candidate_id: None,
        chat_guid: "chat-guid-private".to_owned(),
        anchor_message_guid: "message-guid-private".to_owned(),
        diagnostics: morrow_storage::DiagnosticsTraceLinkage {
            trace_id: None,
            span_id: None,
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
        source_excerpt_policy: FeedbackSourceExcerptPolicy::Hide,
        label_source: FeedbackLabelSource::Provider,
        privacy_tier: FeedbackPrivacyTier::InternalMetadata,
        privacy_metadata_json: "{}".to_owned(),
        created_at: 1_782_352_900,
        expires_at: Some(1_784_944_900),
    }
}

fn native_candidate_draft(anchor_message_guid: &str) -> CandidateDraft {
    CandidateDraft {
        kind: CandidateKind::CalendarEvent,
        chat_guid: "chat-guid-private".to_owned(),
        anchor_message_guid: anchor_message_guid.to_owned(),
        title: "private title".to_owned(),
        confidence_millis: 860,
        normalized_time: "2026-07-15T00:00:00Z".to_owned(),
        evidence_excerpt: "private message excerpt".to_owned(),
        observed_at: 1_782_352_400,
    }
}
