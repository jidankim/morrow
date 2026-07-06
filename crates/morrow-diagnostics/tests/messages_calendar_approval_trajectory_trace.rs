mod phase5_support;

use morrow_diagnostics::{
    validate_trace_record_privacy, TraceComponent, TraceDecision, TraceOperation, TraceOutcome,
    TrajectoryAnchorError, TrajectoryAnchorReport,
};

use phase5_support::{
    anchor_step, assert_serialized_report_is_sanitized, provider_route_record, report_with_step,
    trajectory_records, CASE_ID, DECISION_EVIDENCE_SUBJECT_ID, FEATURE_SNAPSHOT_ID,
};

#[test]
fn trajectory_anchor_report_links_case_snapshot_trace_label_and_decision_evidence(
) -> Result<(), String> {
    // Given: a sanitized Phase 5 trajectory report assembled from existing trace vocabulary.
    let records = trajectory_records();
    let report = TrajectoryAnchorReport {
        trajectory_case_id: CASE_ID.to_owned(),
        steps: records.iter().map(anchor_step).collect(),
    };

    // When: the trajectory anchors are validated and serialized.
    report.validate().map_err(|error| error.to_string())?;
    let serialized = serde_json::to_string(&report).map_err(|error| error.to_string())?;

    // Then: every trajectory step links case -> feature snapshot -> trace/span -> label/outcome -> evidence.
    assert_eq!(report.steps.len(), 4);
    for (step, record) in report.steps.iter().zip(records.iter()) {
        validate_trace_record_privacy(record).map_err(|error| error.to_string())?;
        assert_eq!(step.trajectory_case_id, CASE_ID);
        assert_eq!(step.feature_snapshot_id, FEATURE_SNAPSHOT_ID);
        assert_eq!(step.trace_id, record.trace.trace_id);
        assert_eq!(step.span_id, record.trace.span_id);
        assert_eq!(step.label_type, "proposal_outcome");
        assert_eq!(step.label_value, "accepted");
        assert_eq!(
            step.decision_evidence_subject_id,
            DECISION_EVIDENCE_SUBJECT_ID
        );
        assert_eq!(step.decision_evidence_trace_id, record.trace.trace_id);
        assert_eq!(step.decision_evidence_span_id, record.trace.span_id);
    }
    assert!(report
        .steps
        .iter()
        .any(|step| step.component == TraceComponent::Provider
            && step.operation == TraceOperation::ProviderRoute
            && step.decision == Some(TraceDecision::ProviderRoute)));
    assert!(report
        .steps
        .iter()
        .any(|step| step.component == TraceComponent::Calendar
            && step.operation == TraceOperation::CalendarDryRun
            && step.outcome == TraceOutcome::DryRun));
    assert!(report
        .steps
        .iter()
        .any(|step| step.component == TraceComponent::Correction
            && step.operation == TraceOperation::UserCorrection
            && step.decision == Some(TraceDecision::UserCorrected)));
    assert!(report
        .steps
        .iter()
        .any(|step| step.component == TraceComponent::Replay
            && step.operation == TraceOperation::ReplayRun
            && step.decision == Some(TraceDecision::ReplayCompared)));
    assert_serialized_report_is_sanitized(&serialized)?;
    println!("trajectory_case_id={CASE_ID}");
    println!("feature_snapshot_id={FEATURE_SNAPSHOT_ID}");
    println!("decision_evidence_subject_id={DECISION_EVIDENCE_SUBJECT_ID}");
    Ok(())
}

#[test]
fn trajectory_anchor_report_rejects_mismatched_case_id_and_private_title_marker() {
    // Given: malformed anchors with a mismatched case id and a forbidden raw title token.
    let record = provider_route_record();
    let mut mismatched_case = report_with_step(&record);
    mismatched_case.steps[0].trajectory_case_id = "phase5-case-different".to_owned();
    let mut raw_title = report_with_step(&record);
    raw_title.steps[0].feature_snapshot_id = "raw_title=Provider meeting".to_owned();

    // When / Then: malformed trajectory anchors are rejected before report persistence.
    assert!(matches!(
        mismatched_case.validate(),
        Err(TrajectoryAnchorError::MismatchedCaseId { index: 0 })
    ));
    assert!(matches!(
        raw_title.validate(),
        Err(TrajectoryAnchorError::ForbiddenPrivateToken {
            index: 0,
            field: "feature_snapshot_id"
        })
    ));
    mismatched_case.steps[0].trajectory_case_id = CASE_ID.to_owned();
    raw_title.steps[0].feature_snapshot_id = FEATURE_SNAPSHOT_ID.to_owned();
    assert_eq!(mismatched_case.validate(), Ok(()));
    assert_eq!(raw_title.validate(), Ok(()));
    println!("malformed_case_id_rejected=true");
    println!("private_title_marker_rejected=true");
}

#[test]
fn trajectory_anchor_report_rejects_matching_but_malformed_trace_ids() {
    // Given: a step whose trace and decision-evidence trace IDs agree but do not use local opaque ID shape.
    let record = provider_route_record();
    let mut malformed_trace = report_with_step(&record);
    malformed_trace.steps[0].trace_id = "trace_native-message-guid-private".to_owned();
    malformed_trace.steps[0].decision_evidence_trace_id = malformed_trace.steps[0].trace_id.clone();
    let mut malformed_span = report_with_step(&record);
    malformed_span.steps[0].span_id = "span_0000-not-hex".to_owned();
    malformed_span.steps[0].decision_evidence_span_id = malformed_span.steps[0].span_id.clone();

    // When / Then: matching malformed local IDs are still rejected.
    assert!(matches!(
        malformed_trace.validate(),
        Err(TrajectoryAnchorError::MalformedField {
            index: 0,
            field: "trace_id"
        })
    ));
    assert!(matches!(
        malformed_span.validate(),
        Err(TrajectoryAnchorError::MalformedField {
            index: 0,
            field: "span_id"
        })
    ));
    println!("malformed_matching_trace_id_rejected=true");
    println!("malformed_matching_span_id_rejected=true");
}

#[test]
fn trajectory_anchor_report_rejects_structured_private_boundary_values() {
    // Given: anchors carrying structured private values rather than hardcoded review tokens.
    let record = provider_route_record();

    // When / Then: obvious private payloads are rejected at the anchor boundary.
    for test_case in [
        "feature_snapshot_id|/Users/local/Library/Application Support/Morrow/private.db",
        "decision_evidence_subject_id|message-guid-private",
        "label_value|organizer@example.com",
        "label_value|+1 (415) 555-0199",
        "label_value|Please confirm the provider appointment tomorrow morning.",
        "label_value|{\"provider\":\"codex\",\"title\":\"Provider meeting\"}",
        "label_value|title=Provider meeting",
    ] {
        let (field, private_value) = test_case.split_once('|').expect("private test case shape");
        let mut report = report_with_step(&record);
        match field {
            "feature_snapshot_id" => {
                report.steps[0].feature_snapshot_id = private_value.to_owned();
            }
            "decision_evidence_subject_id" => {
                report.steps[0].decision_evidence_subject_id = private_value.to_owned();
            }
            "label_value" => {
                report.steps[0].label_value = private_value.to_owned();
            }
            _ => unreachable!("test fixture covers all private value fields"),
        }
        assert!(matches!(
            report.validate(),
            Err(TrajectoryAnchorError::ForbiddenPrivateToken { index: 0, field: rejected })
                if rejected == field
        ));
    }
    println!("structured_private_boundary_values_rejected=true");
}
