mod support;

use morrow_storage::{
    DecisionEvidenceSubjectType, FeedbackLabelValue, FeedbackSubjectType, ProposalOutcomeLabel,
    QuietLogDraft, SystemOutcomeLabel,
};
use support::decision_evidence::{candidate_draft, fresh_store, label, meta, snapshot};

#[test]
fn decision_evidence_filters_to_created_candidate_ids_over_recent_unrelated_evidence() {
    // Given
    let (_dir, store) = fresh_store("decision-evidence-created-candidates.sqlite");
    let candidate_id = store
        .create_candidate(candidate_draft())
        .expect("create candidate");
    let subject_id = candidate_id.as_str();
    let mut candidate_meta = meta(FeedbackSubjectType::Candidate, subject_id);
    candidate_meta.candidate_id = Some(candidate_id.clone());
    candidate_meta.created_at = 1_783_000_000;
    candidate_meta.expires_at = Some(1_785_592_000);
    let mut candidate_label = label(
        "candidate-focused-label",
        FeedbackSubjectType::Candidate,
        subject_id,
        FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::Accepted),
    );
    candidate_label.meta = candidate_meta.clone();
    let mut candidate_snapshot = snapshot(
        "candidate-focused-snapshot",
        FeedbackSubjectType::Candidate,
        subject_id,
    );
    candidate_snapshot.meta = candidate_meta;
    record_recent_quiet_evidence(&store);
    store
        .record_label(candidate_label)
        .expect("record candidate label");
    store
        .record_feature_snapshot(candidate_snapshot)
        .expect("record candidate snapshot");

    // When
    let recent = store
        .recent_decision_evidence(1)
        .expect("recent decision evidence");
    let focused = store
        .decision_evidence_for_candidate_ids(&[candidate_id.clone()], 10)
        .expect("focused decision evidence");

    // Then
    assert_eq!(recent.len(), 1);
    assert_eq!(
        recent[0].subject_type,
        DecisionEvidenceSubjectType::QuietLog
    );
    assert_eq!(focused.len(), 1);
    assert_eq!(
        focused[0].subject_type,
        DecisionEvidenceSubjectType::Candidate
    );
    assert_eq!(focused[0].candidate_id.as_ref(), Some(&candidate_id));
}

#[test]
fn decision_evidence_filters_all_candidate_ids_before_result_limit() {
    // Given
    let (_dir, store) = fresh_store("decision-evidence-created-candidate-limit.sqlite");
    let mut first_draft = candidate_draft();
    first_draft.anchor_message_guid = "message-guid-without-evidence".to_owned();
    let first_candidate_id = store
        .create_candidate(first_draft)
        .expect("create first candidate");
    let mut matching_draft = candidate_draft();
    matching_draft.anchor_message_guid = "message-guid-with-evidence".to_owned();
    let matching_candidate_id = store
        .create_candidate(matching_draft)
        .expect("create matching candidate");
    let subject_id = matching_candidate_id.as_str();
    let mut candidate_meta = meta(FeedbackSubjectType::Candidate, subject_id);
    candidate_meta.candidate_id = Some(matching_candidate_id.clone());
    candidate_meta.created_at = 1_783_000_900;
    candidate_meta.expires_at = Some(1_785_592_900);
    let mut candidate_label = label(
        "candidate-after-limit-label",
        FeedbackSubjectType::Candidate,
        subject_id,
        FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::Accepted),
    );
    candidate_label.meta = candidate_meta.clone();
    let mut candidate_snapshot = snapshot(
        "candidate-after-limit-snapshot",
        FeedbackSubjectType::Candidate,
        subject_id,
    );
    candidate_snapshot.meta = candidate_meta;
    store
        .record_label(candidate_label)
        .expect("record candidate label");
    store
        .record_feature_snapshot(candidate_snapshot)
        .expect("record candidate snapshot");

    // When
    let focused = store
        .decision_evidence_for_candidate_ids(
            &[first_candidate_id, matching_candidate_id.clone()],
            1,
        )
        .expect("focused decision evidence");

    // Then
    assert_eq!(focused.len(), 1);
    assert_eq!(
        focused[0].subject_type,
        DecisionEvidenceSubjectType::Candidate
    );
    assert_eq!(
        focused[0].candidate_id.as_ref(),
        Some(&matching_candidate_id)
    );
}

fn record_recent_quiet_evidence(store: &morrow_storage::Store) {
    let subject_id = "quiet-recent-unrelated";
    store
        .record_quiet_log(QuietLogDraft {
            chat_guid: "chat-guid-private".to_owned(),
            anchor_message_guid: "message-guid-private".to_owned(),
            reason: "provider_unavailable".to_owned(),
            excerpt: "Source excerpt hidden by settings.".to_owned(),
            provider_diagnostic: None,
            created_at: 1_783_000_500,
        })
        .expect("record quiet log");
    let mut recent_quiet_meta = meta(FeedbackSubjectType::QuietLog, subject_id);
    recent_quiet_meta.created_at = 1_783_000_500;
    recent_quiet_meta.expires_at = Some(1_785_592_500);
    let mut recent_quiet_label = label(
        "quiet-recent-label",
        FeedbackSubjectType::QuietLog,
        subject_id,
        FeedbackLabelValue::SystemOutcome(SystemOutcomeLabel::FailedProvider),
    );
    recent_quiet_label.meta = recent_quiet_meta.clone();
    let mut recent_quiet_snapshot = snapshot(
        "quiet-recent-snapshot",
        FeedbackSubjectType::QuietLog,
        subject_id,
    );
    recent_quiet_snapshot.meta = recent_quiet_meta;
    recent_quiet_snapshot.route = Some("provider_unavailable".to_owned());
    recent_quiet_snapshot.reason_code = Some("provider_timeout".to_owned());
    recent_quiet_snapshot.confidence_millis = None;
    store
        .record_label(recent_quiet_label)
        .expect("record quiet label");
    store
        .record_feature_snapshot(recent_quiet_snapshot)
        .expect("record quiet snapshot");
}
