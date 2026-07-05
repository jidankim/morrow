mod support;

use morrow_storage::{FeedbackLabelValue, FeedbackSubjectType, ProposalOutcomeLabel};
use support::decision_evidence::{candidate_draft, fresh_store, label, meta, snapshot};

#[test]
fn decision_evidence_summarizes_candidate_source_excerpt() {
    // Given
    let (_dir, store) = fresh_store("decision-evidence-excerpt.sqlite");
    let candidate_id = store
        .create_candidate(candidate_draft())
        .expect("create candidate");
    let subject_id = candidate_id.as_str();
    let mut candidate_meta = meta(FeedbackSubjectType::Candidate, subject_id);
    candidate_meta.candidate_id = Some(candidate_id.clone());
    let mut candidate_label = label(
        "candidate-excerpt-label",
        FeedbackSubjectType::Candidate,
        subject_id,
        FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::Accepted),
    );
    candidate_label.meta = candidate_meta.clone();
    let mut candidate_snapshot = snapshot(
        "candidate-excerpt-snapshot",
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
    assert_eq!(
        summaries[0].excerpt.as_deref(),
        Some("Source excerpt hidden by settings.")
    );
}
