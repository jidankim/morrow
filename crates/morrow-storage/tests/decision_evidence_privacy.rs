mod support;

use morrow_storage::{
    FeedbackLabelValue, FeedbackSubjectType, ProposalOutcomeLabel, StorageError, SystemOutcomeLabel,
};
use support::decision_evidence::{fresh_store, label, snapshot};

#[test]
fn decision_evidence_malformed_diagnostics_rejected_by_existing_validator() {
    // Given
    let (_dir, store) = fresh_store("decision-evidence-malformed-diagnostics.sqlite");
    let mut malformed = label(
        "malformed-diagnostics-label",
        FeedbackSubjectType::Candidate,
        "candidate-subject",
        FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::Accepted),
    );
    malformed.meta.diagnostics.trace_id = Some("chat-guid-raw".to_owned());

    // When
    let error = store
        .record_label(malformed)
        .expect_err("diagnostics should be validated before evidence reads");

    // Then
    match error {
        StorageError::InvalidInput { field, .. } => {
            assert_eq!(field, "diagnostics_trace_id");
        }
        other => panic!("expected invalid diagnostics, got {other}"),
    }
}

#[test]
fn decision_evidence_serialized_summaries_omit_forbidden_raw_fields() {
    // Given
    let (_dir, store) = fresh_store("decision-evidence-forbidden-fields.sqlite");
    let subject_id = "quiet-provider-rejected";
    store
        .record_label(label(
            "privacy-label",
            FeedbackSubjectType::QuietLog,
            subject_id,
            FeedbackLabelValue::SystemOutcome(SystemOutcomeLabel::FailedProvider),
        ))
        .expect("record label");
    store
        .record_feature_snapshot(snapshot(
            "privacy-snapshot",
            FeedbackSubjectType::QuietLog,
            subject_id,
        ))
        .expect("record snapshot");

    // When
    let summaries = store
        .recent_decision_evidence(10)
        .expect("decision evidence");
    let serialized = format!("{summaries:#?}");
    println!("SERIALIZED_SUMMARIES_BEGIN\n{serialized}\nSERIALIZED_SUMMARIES_END");

    // Then
    for forbidden in [
        "raw_text",
        "prompt",
        "response",
        "raw_json",
        "provider_json",
        "full_message",
        "raw_title",
        "title_text",
        "unredacted_title",
        "chat_guid",
        "anchor_message_guid",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "serialized summary contains forbidden field token {forbidden}"
        );
    }
}
