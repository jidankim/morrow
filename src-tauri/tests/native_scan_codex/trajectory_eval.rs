use std::fs;

use morrow_diagnostics::{
    diagnostics_trace_dir, TrajectoryAnchorError, TrajectoryAnchorReport, TrajectoryAnchorStep,
    TrajectoryAnchorStepInput,
};
use morrow_lib::native_bridge::{
    load_decision_evidence_at, DecisionEvidenceTraceRetention, LoadDecisionEvidenceRequest,
    NativeDecisionEvidenceSubjectType,
};
use morrow_storage::Store;

use super::support::{ScanFixture, FIXTURE_MESSAGE_TEXT, NATIVE_CHAT_ID, NATIVE_MESSAGE_ID};
use super::trajectory_eval_support::{
    provider_route_record, record_trajectory_evidence, trace_jsonl, trajectory_records,
    ANCHOR_SPAN_ID, CASE_ID, FEATURE_SNAPSHOT_ID, TRACE_ID,
};

#[test]
fn trajectory_eval_links_case_snapshot_trace_label_outcome_and_decision_evidence(
) -> Result<(), String> {
    // Given: local Phase 5 trajectory evidence with a stored snapshot and retained trace sequence.
    let fixture = ScanFixture::new("codex-trajectory-eval-anchors")?;
    let candidate_id = record_trajectory_evidence(fixture.store_path())?;
    let records = trajectory_records();
    let traces_dir = diagnostics_trace_dir(fixture.app_data_dir());
    fs::create_dir_all(&traces_dir).map_err(|error| error.to_string())?;
    fs::write(
        traces_dir.join("trace-1783000000-0.jsonl"),
        trace_jsonl(&records)?,
    )
    .map_err(|error| error.to_string())?;

    // When: the native decision-evidence read model loads the candidate trajectory anchors.
    let report = load_decision_evidence_at(
        fixture.store_path(),
        fixture.app_data_dir(),
        LoadDecisionEvidenceRequest {
            limit: 10,
            created_candidate_ids: vec![candidate_id.as_str().to_owned()],
        },
    )?;
    let eval_cases = Store::open(fixture.store_path())
        .and_then(|store| store.eval_cases())
        .map_err(|error| error.to_string())?;

    // Then: case id -> feature snapshot id -> trace/span id -> label/outcome -> evidence is linked.
    let eval_case = eval_cases
        .iter()
        .find(|case| case.snapshot.snapshot_key == FEATURE_SNAPSHOT_ID)
        .ok_or_else(|| format!("missing trajectory feature snapshot {FEATURE_SNAPSHOT_ID}"))?;
    assert!(eval_case.snapshot.snapshot_key.contains(CASE_ID));
    assert_eq!(eval_case.snapshot.meta.subject_id, candidate_id.as_str());
    assert_eq!(
        eval_case.snapshot.meta.diagnostics.trace_id.as_deref(),
        Some(TRACE_ID)
    );
    assert_eq!(
        eval_case.snapshot.meta.diagnostics.span_id.as_deref(),
        Some(ANCHOR_SPAN_ID)
    );
    assert_eq!(eval_case.label.label_value.as_str(), "accepted");
    assert_eq!(report.items.len(), 1, "{report:#?}");
    assert_eq!(report.skipped_trace_line_count, 0);
    let item = &report.items[0];
    assert_eq!(
        item.subject_type,
        NativeDecisionEvidenceSubjectType::Candidate
    );
    assert_eq!(item.candidate_id.as_deref(), Some(candidate_id.as_str()));
    assert_eq!(item.route.as_deref(), Some("provider_candidate"));
    assert_eq!(item.reason_code.as_deref(), Some("provider_valid"));
    assert_eq!(item.label_type, "proposal_outcome");
    assert_eq!(item.label_value, "accepted");
    assert!(item.has_diagnostics_hashes);
    assert_eq!(
        item.trace_retention,
        DecisionEvidenceTraceRetention::Retained
    );
    assert_trace_sequence_contains(
        item,
        ExpectedTraceStep::new("provider", "provider_route", Some("provider_route"), "noop"),
    )?;
    assert_trace_sequence_contains(
        item,
        ExpectedTraceStep::new(
            "calendar",
            "calendar_dry_run",
            Some("lifecycle_updated"),
            "dry_run",
        ),
    )?;
    assert_trace_sequence_contains(
        item,
        ExpectedTraceStep::new(
            "correction",
            "user_correction",
            Some("user_corrected"),
            "noop",
        ),
    )?;
    assert_trace_sequence_contains(
        item,
        ExpectedTraceStep::new(
            "replay",
            "replay_run",
            Some("replay_compared"),
            "replay_recorded",
        ),
    )?;
    assert_serialized_report_is_sanitized(
        &serde_json::to_string(&report).map_err(|error| error.to_string())?,
        fixture.app_data_dir().to_string_lossy().as_ref(),
    )?;
    println!("trajectory_eval_case_id={CASE_ID}");
    println!("trajectory_eval_feature_snapshot_id={FEATURE_SNAPSHOT_ID}");
    println!("trajectory_eval_trace_id={TRACE_ID}");
    Ok(())
}

#[test]
fn trajectory_eval_rejects_mismatched_case_id_or_private_title_marker() {
    // Given: malformed local trajectory anchor fixtures.
    let record = provider_route_record();
    let mismatched_case = TrajectoryAnchorReport {
        trajectory_case_id: CASE_ID.to_owned(),
        steps: vec![TrajectoryAnchorStep::from_trace_record(
            TrajectoryAnchorStepInput {
                trajectory_case_id: "phase5-case-mismatch",
                feature_snapshot_id: FEATURE_SNAPSHOT_ID,
                label_type: "proposal_outcome",
                label_value: "accepted",
                decision_evidence_subject_id: "candidate_trajectory_calendar_accept",
                record: &record,
            },
        )],
    };
    let raw_title = TrajectoryAnchorReport {
        trajectory_case_id: CASE_ID.to_owned(),
        steps: vec![TrajectoryAnchorStep::from_trace_record(
            TrajectoryAnchorStepInput {
                trajectory_case_id: CASE_ID,
                feature_snapshot_id: "raw_title=Provider meeting",
                label_type: "proposal_outcome",
                label_value: "accepted",
                decision_evidence_subject_id: "candidate_trajectory_calendar_accept",
                record: &record,
            },
        )],
    };

    // When / Then: malformed trajectory anchors are rejected before native evidence readback.
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
    println!("trajectory_eval_mismatched_case_id_rejected=true");
    println!("trajectory_eval_private_title_marker_rejected=true");
}

#[derive(Debug, Clone, Copy)]
struct ExpectedTraceStep<'a> {
    component: &'a str,
    operation: &'a str,
    decision: Option<&'a str>,
    outcome: &'a str,
}

impl<'a> ExpectedTraceStep<'a> {
    const fn new(
        component: &'a str,
        operation: &'a str,
        decision: Option<&'a str>,
        outcome: &'a str,
    ) -> Self {
        Self {
            component,
            operation,
            decision,
            outcome,
        }
    }
}

fn assert_trace_sequence_contains(
    item: &morrow_lib::native_bridge::DecisionEvidenceItem,
    expected: ExpectedTraceStep<'_>,
) -> Result<(), String> {
    if item.trace_sequence.iter().any(|step| {
        step.component == expected.component
            && step.operation == expected.operation
            && step.decision.as_deref() == expected.decision
            && step.outcome == expected.outcome
    }) {
        Ok(())
    } else {
        Err(format!(
            "decision evidence trace sequence omitted {expected:?}: {item:#?}"
        ))
    }
}

fn assert_serialized_report_is_sanitized(
    serialized: &str,
    app_data_dir: &str,
) -> Result<(), String> {
    for forbidden in [
        FIXTURE_MESSAGE_TEXT,
        "Provider meeting",
        NATIVE_CHAT_ID,
        NATIVE_MESSAGE_ID,
        "raw_title",
        "full_message",
        "provider_json",
        "native_identifier",
        "private trajectory title",
        "private message excerpt",
        app_data_dir,
    ] {
        if serialized.contains(forbidden) {
            return Err(format!(
                "trajectory decision evidence leaked forbidden fixture string: {forbidden}"
            ));
        }
    }
    Ok(())
}
