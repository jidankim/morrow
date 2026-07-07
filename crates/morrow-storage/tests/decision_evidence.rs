#[path = "support/decision_evidence_candidate.rs"]
mod decision_evidence_candidate_support;
#[path = "support/decision_evidence.rs"]
mod decision_evidence_support;

use decision_evidence_candidate_support::candidate_draft;
use decision_evidence_support::{fresh_store, label, meta, snapshot};
use morrow_storage::{
    DecisionEvidenceSubjectType, DecisionEvidenceTraceRetention, DetectionRouteLabel,
    FeedbackLabelType, FeedbackLabelValue, FeedbackPrivacyTier, FeedbackSourceExcerptPolicy,
    FeedbackSubjectType, FieldQualityLabel, ProposalOutcomeLabel, QuietLogDraft,
    SystemOutcomeLabel,
};

#[test]
fn decision_evidence_summarizes_candidate_trace_linkage() {
    // Given
    let (_dir, store) = fresh_store("decision-evidence-candidate.sqlite");
    let candidate_id = store
        .create_candidate(candidate_draft())
        .expect("create candidate");
    let subject_id = candidate_id.as_str();
    let mut candidate_meta = meta(FeedbackSubjectType::Candidate, subject_id);
    candidate_meta.candidate_id = Some(candidate_id.clone());
    let mut candidate_label = label(
        "candidate-label",
        FeedbackSubjectType::Candidate,
        subject_id,
        FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::Accepted),
    );
    candidate_label.meta = candidate_meta.clone();
    let mut candidate_snapshot = snapshot(
        "candidate-snapshot",
        FeedbackSubjectType::Candidate,
        subject_id,
    );
    candidate_snapshot.meta = candidate_meta;

    // When
    store.record_label(candidate_label).expect("record label");
    store
        .record_feature_snapshot(candidate_snapshot)
        .expect("record snapshot");
    let summaries = store
        .recent_decision_evidence(10)
        .expect("decision evidence");

    // Then
    assert_eq!(summaries.len(), 1);
    let summary = &summaries[0];
    assert_eq!(summary.subject_type, DecisionEvidenceSubjectType::Candidate);
    assert_eq!(summary.subject_id, subject_id);
    assert_eq!(summary.candidate_id.as_ref(), Some(&candidate_id));
    assert_eq!(
        summary.candidate_state.map(|state| state.as_str()),
        Some("queued")
    );
    assert_eq!(
        summary.candidate_kind.map(|kind| kind.as_str()),
        Some("calendar_event")
    );
    assert_eq!(summary.route.as_deref(), Some("provider_candidate"));
    assert_eq!(summary.reason_code.as_deref(), Some("provider_valid"));
    assert_eq!(summary.confidence_millis, Some(860));
    assert_eq!(summary.label_type, FeedbackLabelType::ProposalOutcome);
    assert_eq!(
        summary.label_value,
        FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::Accepted)
    );
    assert_eq!(
        summary.source_excerpt_policy,
        FeedbackSourceExcerptPolicy::Hide
    );
    assert_eq!(summary.privacy_tier, FeedbackPrivacyTier::InternalMetadata);
    assert_eq!(
        summary.diagnostics_trace_id.as_deref(),
        Some("trace_018fda8a98bf4cdba33a6f9d42180d6d")
    );
    assert_eq!(
        summary.diagnostics_span_id.as_deref(),
        Some("span_52efdebab8574a5fa6f290831a786e86")
    );
    assert!(summary.diagnostics_chat_hash_present);
    assert!(summary.diagnostics_message_hash_present);
    assert_eq!(
        summary.trace_retention,
        DecisionEvidenceTraceRetention::NotChecked
    );
}

#[test]
fn decision_evidence_summarizes_quiet_trace_linkage() {
    // Given
    let (_dir, store) = fresh_store("decision-evidence-quiet.sqlite");
    let subject_id = "quiet-provider-unavailable";
    store
        .record_quiet_log(QuietLogDraft {
            chat_guid: "chat-guid-private".to_owned(),
            anchor_message_guid: "message-guid-private".to_owned(),
            reason: "provider_unavailable".to_owned(),
            excerpt: "Source excerpt hidden by settings.".to_owned(),
            provider_diagnostic: Some("stale provider diagnostic".to_owned()),
            created_at: 1_782_999_999,
        })
        .expect("record stale quiet log");
    store
        .record_quiet_log(QuietLogDraft {
            chat_guid: "chat-guid-private".to_owned(),
            anchor_message_guid: "message-guid-private".to_owned(),
            reason: "provider_unavailable".to_owned(),
            excerpt: "Source excerpt hidden by settings.".to_owned(),
            provider_diagnostic: Some("codex provider command timed out".to_owned()),
            created_at: 1_783_000_000,
        })
        .expect("record quiet log");
    let quiet_label = label(
        "quiet-label",
        FeedbackSubjectType::QuietLog,
        subject_id,
        FeedbackLabelValue::SystemOutcome(SystemOutcomeLabel::FailedProvider),
    );
    let mut quiet_snapshot = snapshot("quiet-snapshot", FeedbackSubjectType::QuietLog, subject_id);
    quiet_snapshot.route = Some("provider_unavailable".to_owned());
    quiet_snapshot.reason_code = Some("provider_unavailable".to_owned());
    quiet_snapshot.confidence_millis = None;

    // When
    store.record_label(quiet_label).expect("record label");
    store
        .record_feature_snapshot(quiet_snapshot)
        .expect("record snapshot");
    let summaries = store
        .recent_decision_evidence(10)
        .expect("decision evidence");

    // Then
    assert_eq!(summaries.len(), 1);
    let summary = &summaries[0];
    assert_eq!(summary.subject_type, DecisionEvidenceSubjectType::QuietLog);
    assert_eq!(summary.subject_id, subject_id);
    assert!(summary.candidate_id.is_none());
    assert!(summary.candidate_state.is_none());
    assert!(summary.candidate_kind.is_none());
    assert_eq!(summary.route.as_deref(), Some("provider_unavailable"));
    assert_eq!(summary.reason_code.as_deref(), Some("provider_unavailable"));
    assert_eq!(
        summary.provider_diagnostic.as_deref(),
        Some("codex provider command timed out")
    );
    assert_eq!(summary.confidence_millis, None);
    assert_eq!(summary.label_type, FeedbackLabelType::SystemOutcome);
    assert_eq!(
        summary.label_value,
        FeedbackLabelValue::SystemOutcome(SystemOutcomeLabel::FailedProvider)
    );
    assert_eq!(
        summary.diagnostics_trace_id.as_deref(),
        Some("trace_018fda8a98bf4cdba33a6f9d42180d6d")
    );
    assert_eq!(
        summary.diagnostics_span_id.as_deref(),
        Some("span_52efdebab8574a5fa6f290831a786e86")
    );
    assert!(summary.diagnostics_chat_hash_present);
    assert!(summary.diagnostics_message_hash_present);
    assert_eq!(
        summary.trace_retention,
        DecisionEvidenceTraceRetention::NotChecked
    );
}

#[test]
fn decision_evidence_uses_latest_label_without_duplicate_snapshot_rows() {
    // Given
    let (_dir, store) = fresh_store("decision-evidence-latest-label.sqlite");
    let candidate_id = store
        .create_candidate(candidate_draft())
        .expect("create candidate");
    let subject_id = candidate_id.as_str();
    let mut candidate_meta = meta(FeedbackSubjectType::Candidate, subject_id);
    candidate_meta.candidate_id = Some(candidate_id.clone());
    let mut first_label = label(
        "candidate-label-first",
        FeedbackSubjectType::Candidate,
        subject_id,
        FeedbackLabelValue::DetectionRoute(DetectionRouteLabel::ProviderCandidate),
    );
    first_label.meta = candidate_meta.clone();
    let mut second_label = label(
        "candidate-label-second",
        FeedbackSubjectType::Candidate,
        subject_id,
        FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::Accepted),
    );
    second_label.meta = candidate_meta.clone();
    let mut candidate_snapshot = snapshot(
        "candidate-snapshot",
        FeedbackSubjectType::Candidate,
        subject_id,
    );
    candidate_snapshot.meta = candidate_meta;

    // When
    store.record_label(first_label).expect("record first label");
    store
        .record_label(second_label)
        .expect("record second label");
    store
        .record_feature_snapshot(candidate_snapshot)
        .expect("record snapshot");
    let summaries = store
        .recent_decision_evidence(10)
        .expect("decision evidence");

    // Then
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].label_type, FeedbackLabelType::ProposalOutcome);
    assert_eq!(
        summaries[0].label_value,
        FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::Accepted)
    );
}

#[test]
fn decision_evidence_summarizes_candidate_correction_label_when_snapshot_exists() {
    // Given
    let (_dir, store) = fresh_store("decision-evidence-correction-label.sqlite");
    let candidate_id = store
        .create_candidate(candidate_draft())
        .expect("create candidate");
    let subject_id = candidate_id.as_str();
    let mut candidate_meta = meta(FeedbackSubjectType::Candidate, subject_id);
    candidate_meta.candidate_id = Some(candidate_id.clone());
    let mut proposal_label = label(
        "candidate-correction-proposal-label",
        FeedbackSubjectType::Candidate,
        subject_id,
        FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::PendingEdited),
    );
    proposal_label.meta = candidate_meta.clone();
    let mut field_quality_label = label(
        "candidate-correction-field-label",
        FeedbackSubjectType::Candidate,
        subject_id,
        FeedbackLabelValue::FieldQuality(FieldQualityLabel::TitleEdited),
    );
    field_quality_label.meta = candidate_meta.clone();
    let mut candidate_snapshot = snapshot(
        "candidate-correction-snapshot",
        FeedbackSubjectType::Candidate,
        subject_id,
    );
    candidate_snapshot.meta = candidate_meta;
    candidate_snapshot.route = Some("human_correction".to_owned());
    candidate_snapshot.reason_code = Some("user_correction_applied".to_owned());

    // When
    store
        .record_label(proposal_label)
        .expect("record proposal outcome label");
    store
        .record_label(field_quality_label)
        .expect("record field quality label");
    store
        .record_feature_snapshot(candidate_snapshot)
        .expect("record snapshot");
    let summaries = store
        .recent_decision_evidence(10)
        .expect("decision evidence");

    // Then
    assert_eq!(summaries.len(), 1);
    let summary = &summaries[0];
    assert_eq!(summary.subject_type, DecisionEvidenceSubjectType::Candidate);
    assert_eq!(summary.candidate_id.as_ref(), Some(&candidate_id));
    assert_eq!(summary.route.as_deref(), Some("human_correction"));
    assert_eq!(
        summary.reason_code.as_deref(),
        Some("user_correction_applied")
    );
    assert_eq!(summary.label_type, FeedbackLabelType::FieldQuality);
    assert_eq!(
        summary.label_value,
        FeedbackLabelValue::FieldQuality(FieldQualityLabel::TitleEdited)
    );
    assert_eq!(
        summary.diagnostics_trace_id.as_deref(),
        Some("trace_018fda8a98bf4cdba33a6f9d42180d6d")
    );
    assert_eq!(
        summary.trace_retention,
        DecisionEvidenceTraceRetention::NotChecked
    );
}
