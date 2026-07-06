#![doc = "Phase 5 Messages-to-Calendar approval trajectory schema tests."]

use std::collections::BTreeSet;

use morrow_storage::CandidateKind;

mod phase5_support;

use phase5_support::contracts::{
    LabelContract, LABEL_ACCEPTED, LABEL_CANARY, LABEL_EDITED, LABEL_QUIET, LABEL_REJECTED,
    OUTCOME_ACCEPTED, OUTCOME_CANARY, OUTCOME_COLLATERAL, OUTCOME_EDITED, OUTCOME_QUIET,
    OUTCOME_REJECTED, OUTCOME_REPLAY, TRACE_ACCEPTED, TRACE_CANARY, TRACE_EDITED, TRACE_QUIET,
    TRACE_REJECTED,
};
use phase5_support::fixtures::TRAJECTORY_CASES;
use phase5_support::local_runner::{
    run_local_trajectory_cases, run_local_trajectory_cases_with_live_attempt, with_out_dir_env_lock,
};
use phase5_support::schema::{
    AllowedSideEffect, CleanupAction, ExpectedOutcome, ProposalKindExpectation, ScoreWeight,
    TrajectoryCase,
};
use phase5_support::validate_trajectory_cases;

#[test]
fn schema_fixtures_validate_privacy_safe_contracts() -> Result<(), Box<dyn std::error::Error>> {
    validate_trajectory_cases(&TRAJECTORY_CASES)?;

    let families = TRAJECTORY_CASES
        .iter()
        .map(|case| case.family)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        families,
        BTreeSet::from([
            "scheduled_meeting_accepted",
            "scheduled_meeting_rejected",
            "scheduled_meeting_edited_before_approval",
            "task_reminder_accepted",
            "task_reminder_rejected",
            "provider_quiet_low_confidence",
            "collateral_damage_non_target_preserved",
            "replay_idempotent_retry",
            "privacy_canary_rejection",
        ])
    );

    for case in TRAJECTORY_CASES {
        assert_case_contract(case)?;
    }

    println!(
        "PASS schema assertions: stable ids privacy-safe fields operations labels outcomes collateral cleanup score_weights"
    );
    Ok(())
}

#[test]
fn schema_validator_rejects_malformed_input_raw_missing_cleanup_and_duplicate_ids() {
    let mut raw_canary = TRAJECTORY_CASES[0];
    raw_canary.bounded_excerpt_placeholder = "[PHASE5_RAW_MESSAGE_PROVIDER_CANARY_DO_NOT_STORE]";
    assert_validation_error([raw_canary], "forbidden raw marker");

    let mut missing_cleanup = TRAJECTORY_CASES[0];
    missing_cleanup.cleanup = None;
    assert_validation_error([missing_cleanup], "missing cleanup expectation");

    let duplicate = [TRAJECTORY_CASES[0], TRAJECTORY_CASES[0]];
    assert_validation_error(duplicate, "duplicate trajectory_case_id");

    println!(
        "PASS schema malformed_input assertions: raw canary missing cleanup duplicate id rejected"
    );
}

fn assert_case_contract(case: TrajectoryCase) -> Result<(), Box<dyn std::error::Error>> {
    assert!(case.trajectory_case_id.starts_with("phase5:"));
    assert!(case.trajectory_case_id.ends_with(":v1"));
    assert_expected_labels_and_outcomes(case);
    assert_eq!(score_total(case.score_weights), 100);
    assert_expected_operations(case);
    assert_expected_kind(case);
    assert_cleanup_contract(case)?;
    Ok(())
}

fn assert_expected_labels_and_outcomes(case: TrajectoryCase) {
    match case.family {
        "scheduled_meeting_accepted" | "task_reminder_accepted" => {
            assert_contract(case, &OUTCOME_ACCEPTED, &LABEL_ACCEPTED);
        }
        "scheduled_meeting_rejected" | "task_reminder_rejected" => {
            assert_contract(case, &OUTCOME_REJECTED, &LABEL_REJECTED);
        }
        "scheduled_meeting_edited_before_approval" => {
            assert_contract(case, &OUTCOME_EDITED, &LABEL_EDITED)
        }
        "provider_quiet_low_confidence" => {
            assert_contract(case, &OUTCOME_QUIET, &LABEL_QUIET);
        }
        "collateral_damage_non_target_preserved" => {
            assert_contract(case, &OUTCOME_COLLATERAL, &LABEL_ACCEPTED)
        }
        "replay_idempotent_retry" => assert_contract(case, &OUTCOME_REPLAY, &LABEL_ACCEPTED),
        "privacy_canary_rejection" => assert_contract(case, &OUTCOME_CANARY, &LABEL_CANARY),
        other => panic!("unexpected fixture family {other}"),
    }
}

fn assert_contract(case: TrajectoryCase, outcomes: &[ExpectedOutcome], labels: &[LabelContract]) {
    let actual_labels = case
        .expected_labels
        .iter()
        .map(|label| (label.label_type, label.label_value))
        .collect::<Vec<_>>();
    assert_eq!(case.expected_outcomes, outcomes);
    assert_eq!(actual_labels, labels);
}

fn assert_expected_operations(case: TrajectoryCase) {
    let operations = case
        .expected_trace_operations
        .iter()
        .map(|operation| {
            (
                operation.component,
                operation.operation,
                operation.outcome,
                operation.privacy_tier,
            )
        })
        .collect::<Vec<_>>();
    match case.family {
        "scheduled_meeting_accepted"
        | "task_reminder_accepted"
        | "collateral_damage_non_target_preserved"
        | "replay_idempotent_retry" => assert_eq!(operations, TRACE_ACCEPTED),
        "scheduled_meeting_rejected" | "task_reminder_rejected" => {
            assert_eq!(operations, TRACE_REJECTED);
        }
        "scheduled_meeting_edited_before_approval" => assert_eq!(operations, TRACE_EDITED),
        "provider_quiet_low_confidence" => assert_eq!(operations, TRACE_QUIET),
        "privacy_canary_rejection" => assert_eq!(operations, TRACE_CANARY),
        other => panic!("unexpected fixture family {other}"),
    }
}

fn assert_expected_kind(case: TrajectoryCase) {
    match case.family {
        "task_reminder_accepted" | "task_reminder_rejected" => {
            assert_eq!(case.expected_candidate_kind, CandidateKind::TaskReminder);
            assert_eq!(
                case.expected_proposal_kind,
                ProposalKindExpectation::TaskReminder
            );
        }
        "provider_quiet_low_confidence" | "privacy_canary_rejection" => {
            assert_eq!(
                case.expected_proposal_kind,
                ProposalKindExpectation::NoProposal
            );
        }
        _ => {
            assert_eq!(case.expected_candidate_kind, CandidateKind::CalendarEvent);
            assert_eq!(
                case.expected_proposal_kind,
                ProposalKindExpectation::CalendarEvent
            );
        }
    }
}

fn assert_cleanup_contract(case: TrajectoryCase) -> Result<(), Box<dyn std::error::Error>> {
    let cleanup = case
        .cleanup
        .ok_or_else(|| std::io::Error::other("schema fixture missing cleanup expectation"))?;
    assert!(!cleanup.live_surface_used);
    match case.family {
        "provider_quiet_low_confidence" | "privacy_canary_rejection" => {
            assert_eq!(cleanup.action, CleanupAction::NoExternalCreated);
            assert!(case
                .allowed_side_effects
                .contains(&AllowedSideEffect::RejectBeforeMutation));
        }
        "collateral_damage_non_target_preserved" => {
            assert_eq!(cleanup.action, CleanupAction::PreserveNonTargetOnly);
            assert!(cleanup.non_target_fixture_id.is_some());
            assert!(case
                .expected_outcomes
                .contains(&ExpectedOutcome::NonTargetPreserved));
            assert!(case
                .allowed_side_effects
                .contains(&AllowedSideEffect::PreserveNonTargetExternal));
        }
        _ => assert_eq!(cleanup.action, CleanupAction::DeleteProposedExternal),
    }
    Ok(())
}

fn score_total(weights: &[ScoreWeight]) -> u16 {
    weights
        .iter()
        .fold(0_u16, |sum, weight| sum.saturating_add(weight.weight))
}

fn assert_validation_error<const N: usize>(cases: [TrajectoryCase; N], expected: &str) {
    let err = validate_trajectory_cases(&cases)
        .err()
        .unwrap_or_else(|| panic!("expected schema validation error containing {expected}"));
    assert!(
        err.to_string().contains(expected),
        "expected {expected}, got {err}"
    );
}

#[test]
fn runner_executes_all_fixture_backed_cases_and_emits_sanitized_artifact(
) -> Result<(), Box<dyn std::error::Error>> {
    let artifact = with_out_dir_env_lock(run_local_trajectory_cases)?;

    assert_eq!(artifact.case_results.len(), TRAJECTORY_CASES.len());
    for case in TRAJECTORY_CASES {
        assert!(artifact
            .case_results
            .iter()
            .any(|result| result.trajectory_case_id == case.trajectory_case_id));
    }
    assert!(artifact.cleanup_proof.cleaned);
    assert!(artifact.live_surface_proof.no_live_surfaces_used);
    assert!(artifact.storage_readback.label_count >= 20);
    assert_eq!(
        artifact.storage_readback.feature_snapshot_count,
        i64::try_from(artifact.case_results.len())?
    );
    assert_eq!(
        artifact.storage_readback.decision_evidence_count,
        artifact.case_results.len()
    );
    assert_eq!(artifact.feedback_eval.status, "failed");
    assert!(artifact.feedback_eval.cases_evaluated > 0);
    assert_eq!(artifact.feedback_eval.cases_skipped, 1);
    assert!(!artifact
        .serialized
        .contains("PHASE5_RAW_MESSAGE_PROVIDER_CANARY_DO_NOT_STORE"));
    assert!(!artifact.serialized.contains("/Users/"));
    assert!(!artifact.serialized.contains("/private/"));
    println!("local_runner_artifact=local-trajectory-run.json");
    println!(
        "local_runner_cases={}",
        artifact
            .case_results
            .iter()
            .map(|result| result.trajectory_case_id.as_str())
            .collect::<Vec<_>>()
            .join(",")
    );
    Ok(())
}

#[test]
fn runner_rejects_test_dependency_live_surface_attempt_and_records_rejection(
) -> Result<(), Box<dyn std::error::Error>> {
    let rejection = with_out_dir_env_lock(run_local_trajectory_cases_with_live_attempt)?;

    assert!(rejection.rejected);
    assert!(rejection.attempted);
    assert!(rejection.blocked_before_side_effect);
    assert_eq!(rejection.boundary, "local_runner_live_surface_guard");
    assert_eq!(rejection.surface, "messages_sqlite");
    assert_eq!(rejection.sanitized_target, "[REDACTED_LOCAL_PATH]");
    assert!(rejection.serialized.contains("\"rejected\": true"));
    assert!(rejection.serialized.contains("\"attempted\": true"));
    assert!(rejection.serialized.contains("messages_sqlite"));
    assert!(!rejection.serialized.contains("/Users/"));
    assert!(!rejection.serialized.contains("/private/"));
    println!("local_runner_live_surface_rejection=live-surface-rejection.json");
    Ok(())
}
