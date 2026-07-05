use std::fs;

use morrow_diagnostics::diagnostics_trace_dir;
use morrow_lib::native_bridge::{
    load_decision_evidence_at, DecisionEvidenceTraceRetention, LoadDecisionEvidenceRequest,
    NativeDecisionEvidenceSubjectType,
};
use morrow_storage::{
    CandidateDraft, CandidateId, CandidateKind, DiagnosticsTraceLinkage, FeatureSnapshot,
    FeedbackLabelSource, FeedbackLabelValue, FeedbackPrivacyTier, FeedbackRecordMeta,
    FeedbackSourceExcerptPolicy, FeedbackSubjectType, FieldQualityLabel, Label, Store,
    FEEDBACK_EVAL_SCHEMA_VERSION,
};

use super::support::{
    ScanFixture, FIXTURE_MESSAGE_TEXT, NATIVE_CHAT_ID, NATIVE_MESSAGE_ID, PRIVACY_CANARY,
};

const PHASE4_TRACE_ID: &str = "trace_00000000000000000000000000000041";
const PHASE4_SPAN_ID: &str = "span_00000000000000000000000000000041";
const PHASE4_CORRECTION_TRACE_JSONL: &str = include_str!(
    "../../../crates/morrow-diagnostics/fixtures/human_approval_correction_trace_v1_snapshot.jsonl"
);

#[test]
fn decision_evidence_surfaces_phase4_candidate_correction_trace() -> Result<(), String> {
    // Given
    let fixture = ScanFixture::new("codex-decision-evidence-phase4-correction")?;
    let candidate_id = record_phase4_correction_evidence(fixture.store_path())?;
    let traces_dir = diagnostics_trace_dir(fixture.app_data_dir());
    fs::create_dir_all(&traces_dir).map_err(|error| error.to_string())?;
    fs::write(
        traces_dir.join("trace-1783000000-0.jsonl"),
        PHASE4_CORRECTION_TRACE_JSONL,
    )
    .map_err(|error| error.to_string())?;

    // When
    let report = load_decision_evidence_at(
        fixture.store_path(),
        fixture.app_data_dir(),
        LoadDecisionEvidenceRequest {
            limit: 10,
            created_candidate_ids: vec![candidate_id.as_str().to_owned()],
        },
    )?;
    copy_phase4_report_if_requested("phase4-correction-decision-evidence.json", &report)?;

    // Then
    assert_eq!(report.items.len(), 1, "{report:#?}");
    assert_eq!(report.skipped_trace_line_count, 0);
    let item = &report.items[0];
    assert_eq!(
        item.subject_type,
        NativeDecisionEvidenceSubjectType::Candidate
    );
    assert_eq!(item.candidate_id.as_deref(), Some(candidate_id.as_str()));
    assert_eq!(item.route.as_deref(), Some("human_correction"));
    assert_eq!(item.reason_code.as_deref(), Some("user_correction_applied"));
    assert_eq!(item.label_type, "field_quality");
    assert_eq!(item.label_value, "title_edited");
    assert_eq!(
        item.source_excerpt.as_deref(),
        Some("Source excerpt hidden by settings.")
    );
    assert!(item.has_diagnostics_hashes);
    assert_eq!(
        item.trace_retention,
        DecisionEvidenceTraceRetention::Retained
    );
    assert!(item
        .trace_sequence
        .iter()
        .any(|step| step.component == "correction"
            && step.operation == "user_correction"
            && step.decision.as_deref() == Some("user_corrected")));

    let serialized = serde_json::to_string(&report).map_err(|error| error.to_string())?;
    let serialized_value = serde_json::from_str::<serde_json::Value>(&serialized)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        serialized_value["items"][0]["sourceExcerpt"],
        "Source excerpt hidden by settings."
    );
    assert_phase4_serialized_report_is_sanitized(
        &serialized,
        fixture.app_data_dir().to_string_lossy().as_ref(),
    )?;
    println!("phase4_decision_evidence_artifact=phase4-correction-decision-evidence.json");
    println!("operation=user_correction");
    println!("decision=user_corrected");
    println!("label_type=field_quality");
    Ok(())
}

fn record_phase4_correction_evidence(store_path: &std::path::Path) -> Result<CandidateId, String> {
    let store = Store::open(store_path).map_err(|error| error.to_string())?;
    let candidate_id = store
        .create_candidate(CandidateDraft {
            kind: CandidateKind::CalendarEvent,
            chat_guid: "chat-guid-private".to_owned(),
            anchor_message_guid: "message-guid-private".to_owned(),
            title: "private corrected title".to_owned(),
            confidence_millis: 860,
            normalized_time: "2026-07-15T00:00:00Z".to_owned(),
            evidence_excerpt: "private message excerpt".to_owned(),
            observed_at: 1_783_000_000,
        })
        .map_err(|error| error.to_string())?;
    let subject_id = candidate_id.as_str();
    let meta = phase4_candidate_meta(subject_id, &candidate_id);
    store
        .record_label(Label {
            id: None,
            label_key: "phase4-correction-field-quality".to_owned(),
            label_value: FeedbackLabelValue::FieldQuality(FieldQualityLabel::TitleEdited),
            meta: meta.clone(),
        })
        .map_err(|error| error.to_string())?;
    store
        .record_feature_snapshot(FeatureSnapshot {
            id: None,
            snapshot_key: "phase4-correction-snapshot".to_owned(),
            meta,
            route: Some("human_correction".to_owned()),
            reason_code: Some("user_correction_applied".to_owned()),
            confidence_millis: Some(860),
            participant_count: Some(2),
            tapback_signal: Some("absent".to_owned()),
            sender_signal_available: false,
            context_window_available: false,
            excerpt: Some("Source excerpt hidden by settings.".to_owned()),
        })
        .map_err(|error| error.to_string())?;
    Ok(candidate_id)
}

fn phase4_candidate_meta(subject_id: &str, candidate_id: &CandidateId) -> FeedbackRecordMeta {
    FeedbackRecordMeta {
        schema_version: FEEDBACK_EVAL_SCHEMA_VERSION,
        subject_type: FeedbackSubjectType::Candidate,
        subject_id: subject_id.to_owned(),
        candidate_id: Some(candidate_id.clone()),
        chat_guid: "chat-guid-private".to_owned(),
        anchor_message_guid: "message-guid-private".to_owned(),
        diagnostics: DiagnosticsTraceLinkage {
            trace_id: Some(PHASE4_TRACE_ID.to_owned()),
            span_id: Some(PHASE4_SPAN_ID.to_owned()),
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
        label_source: FeedbackLabelSource::ManualAlpha,
        privacy_tier: FeedbackPrivacyTier::HashedIdentifier,
        privacy_metadata_json: "{}".to_owned(),
        created_at: 1_783_000_000,
        expires_at: Some(1_785_592_000),
    }
}

fn assert_phase4_serialized_report_is_sanitized(
    serialized: &str,
    app_data_dir: &str,
) -> Result<(), String> {
    for required in [
        "user_correction",
        "user_corrected",
        "field_quality",
        "title_edited",
    ] {
        if !serialized.contains(required) {
            return Err(format!(
                "phase4 decision evidence omitted required token {required}: {serialized}"
            ));
        }
    }
    for forbidden in [
        FIXTURE_MESSAGE_TEXT,
        "Provider meeting",
        NATIVE_CHAT_ID,
        NATIVE_MESSAGE_ID,
        PRIVACY_CANARY,
        "MORROW_PRIVACY_CANARY_RAW_CORRECTION",
        "corrected title",
        "raw message",
        "native_identifier",
        app_data_dir,
    ] {
        if serialized.contains(forbidden) {
            return Err(format!(
                "phase4 decision evidence leaked forbidden fixture string: {forbidden}"
            ));
        }
    }
    Ok(())
}

fn copy_phase4_report_if_requested(
    name: &str,
    report: &morrow_lib::native_bridge::DecisionEvidenceReport,
) -> Result<(), String> {
    let Ok(dir) = std::env::var("MORROW_PHASE4_DECISION_EVIDENCE_DIR") else {
        return Ok(());
    };
    let dir = std::path::PathBuf::from(dir);
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let serialized = serde_json::to_string(report).map_err(|error| error.to_string())?;
    fs::write(dir.join(name), serialized).map_err(|error| error.to_string())
}
