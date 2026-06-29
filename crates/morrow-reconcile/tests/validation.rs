//! Reconciliation boundary validation tests.

use morrow_reconcile::{
    reconcile_candidate, CandidateLifecycle, ExternalItemObservation, ReconcileError,
};
use morrow_storage::{
    CandidateId, CandidateKind, CandidateState, ExternalObjectMapping, ExternalSource,
};
use std::io::Error;

#[path = "../examples/reconcile_smoke/summary.rs"]
/// Reused smoke summary validation fixture.
pub mod smoke_summary;

#[test]
fn rejects_malformed_external_observations_when_boundary_fields_are_blank(
) -> Result<(), Box<dyn std::error::Error>> {
    for (observation, expected_field) in [
        (
            ExternalItemObservation::approved_by_move(" ", "source-main"),
            "external_object_id",
        ),
        (
            ExternalItemObservation::approved_by_move("real-calendar", "\t"),
            "external_source_id",
        ),
        (
            ExternalItemObservation::approved_by_copy("\n", "source-main"),
            "external_object_id",
        ),
        (
            ExternalItemObservation::approved_by_copy("real-copy", " "),
            "external_source_id",
        ),
        (
            ExternalItemObservation::PendingEdited {
                observed_title: "  ".to_owned(),
            },
            "observed_title",
        ),
    ] {
        let err = reconcile_candidate(
            &visible_lifecycle(CandidateKind::CalendarEvent, CandidateState::Visible),
            &observation,
        )
        .err()
        .ok_or_else(|| Error::other("missing expected error"))?;

        assert_invalid_observation(&err, expected_field);
    }
    Ok(())
}

#[test]
fn rejects_mapping_when_mapping_candidate_does_not_own_lifecycle(
) -> Result<(), Box<dyn std::error::Error>> {
    let expected = CandidateId::derive(
        CandidateKind::CalendarEvent,
        "chat-owner",
        "message-owner",
        "2026-07-01T10:00:00Z",
    );
    let actual = CandidateId::derive(
        CandidateKind::CalendarEvent,
        "chat-foreign",
        "message-foreign",
        "2026-07-01T10:00:00Z",
    );
    let candidate = CandidateLifecycle {
        candidate_id: expected.clone(),
        kind: CandidateKind::CalendarEvent,
        state: CandidateState::Visible,
        mapping: Some(mapping(&actual, "foreign-proposed")),
        observed_at: 90,
    };

    let err = reconcile_candidate(&candidate, &ExternalItemObservation::Pending)
        .err()
        .ok_or_else(|| Error::other("missing expected error"))?;

    assert!(matches!(
        err,
        ReconcileError::InvalidMappingCandidate {
            expected: ref expected_id,
            actual: ref actual_id,
        } if expected_id == &expected.to_string() && actual_id == &actual.to_string()
    ));
    Ok(())
}

#[test]
fn rejects_zero_readback_counts_before_smoke_pass() -> Result<(), Box<dyn std::error::Error>> {
    let err = smoke_summary::write_summary(&smoke_summary::SmokeCounts::default())
        .err()
        .ok_or_else(|| Error::other("missing expected error"))?;

    assert!(err.to_string().contains("approved_by_move"));
    Ok(())
}

fn assert_invalid_observation(err: &ReconcileError, expected_field: &str) {
    assert!(matches!(
        err,
        ReconcileError::InvalidObservation {
            field,
            ref reason,
        } if *field == expected_field && reason == "must not be empty"
    ));
}

fn visible_lifecycle(kind: CandidateKind, state: CandidateState) -> CandidateLifecycle {
    CandidateLifecycle {
        candidate_id: CandidateId::derive(kind, "chat", "message", "2026-07-01T10:00:00Z"),
        kind,
        state,
        mapping: None,
        observed_at: 10,
    }
}

fn mapping(candidate_id: &CandidateId, external_object_id: &str) -> ExternalObjectMapping {
    ExternalObjectMapping {
        candidate_id: candidate_id.clone(),
        source: ExternalSource::Calendar,
        external_object_id: external_object_id.to_owned(),
        external_source_id: "source-main".to_owned(),
        mapped_at: 1,
    }
}
