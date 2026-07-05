use std::path::Path;
use std::process::Command;

use morrow_reconcile::{
    reconcile_candidate, CandidateLifecycle, ExternalItemObservation, LifecycleReason,
    ReconcileError,
};
use morrow_storage::{CandidateId, CandidateKind, CandidateState, ExternalObjectMapping, Store};

use crate::support::TestError;

pub(super) fn visible_plan(
    candidate_id: &CandidateId,
    kind: CandidateKind,
    mapping: Option<ExternalObjectMapping>,
    observed_at: i64,
    observation: &ExternalItemObservation,
) -> Result<morrow_reconcile::ReconciliationPlan, TestError> {
    reconcile_candidate(
        &CandidateLifecycle {
            candidate_id: candidate_id.clone(),
            kind,
            state: CandidateState::Visible,
            mapping,
            observed_at,
        },
        observation,
    )
    .map_err(TestError::from)
}

pub(super) fn lifecycle(state: CandidateState) -> CandidateLifecycle {
    CandidateLifecycle {
        candidate_id: CandidateId::derive(
            CandidateKind::CalendarEvent,
            "phase4-chat",
            "phase4-message",
            "2026-07-01T10:00:00Z",
        ),
        kind: CandidateKind::CalendarEvent,
        state,
        mapping: None,
        observed_at: 10,
    }
}

pub(super) fn assert_invalid_observation(
    err: &ReconcileError,
    expected_field: &str,
) -> Result<(), TestError> {
    match err {
        ReconcileError::InvalidObservation { field, .. } if *field == expected_field => Ok(()),
        other => Err(TestError::Command(format!(
            "expected invalid observation field {expected_field}, got {other}"
        ))),
    }
}

pub(super) fn assert_audit_reason_count(
    store: &Store,
    candidate_id: &CandidateId,
    reason: LifecycleReason,
    expected: usize,
) -> Result<(), TestError> {
    let actual = store
        .audit_entries(candidate_id)?
        .iter()
        .filter(|entry| entry.reason == reason.as_str())
        .count();
    if actual != expected {
        return Err(TestError::Command(format!(
            "expected audit reason {} count {expected}, got {actual}",
            reason.as_str()
        )));
    }
    Ok(())
}

pub(super) fn assert_feedback_label_count(
    db_path: &Path,
    label_type: &str,
    label_value: &str,
    expected: i64,
) -> Result<(), TestError> {
    let sql = format!(
        "SELECT COUNT(*) FROM labels WHERE label_type = '{label_type}' AND label_value = '{label_value}';"
    );
    let actual = sqlite_count(db_path, &sql)?;
    if actual != expected {
        return Err(TestError::Command(format!(
            "expected feedback label {label_type}={label_value} count {expected}, got {actual}"
        )));
    }
    Ok(())
}

pub(super) fn assert_feedback_text_absent(db_path: &Path, value: &str) -> Result<(), TestError> {
    let sql = format!(
        "SELECT COUNT(*) FROM feedback_events WHERE event_key LIKE '%{value}%' OR privacy_metadata_json LIKE '%{value}%' UNION ALL SELECT COUNT(*) FROM labels WHERE label_key LIKE '%{value}%' OR privacy_metadata_json LIKE '%{value}%';"
    );
    let actual = sqlite_count_sum(db_path, &sql)?;
    if actual != 0 {
        return Err(TestError::Command(format!(
            "expected feedback metadata to omit raw corrected value {value}"
        )));
    }
    Ok(())
}

fn sqlite_count(db_path: &Path, sql: &str) -> Result<i64, TestError> {
    let output = Command::new("sqlite3")
        .arg(db_path)
        .arg(sql)
        .output()
        .map_err(|err| TestError::Command(err.to_string()))?;
    if !output.status.success() {
        return Err(TestError::Command(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<i64>()
        .map_err(|err| TestError::Command(err.to_string()))
}

fn sqlite_count_sum(db_path: &Path, sql: &str) -> Result<i64, TestError> {
    let output = Command::new("sqlite3")
        .arg(db_path)
        .arg(sql)
        .output()
        .map_err(|err| TestError::Command(err.to_string()))?;
    if !output.status.success() {
        return Err(TestError::Command(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .try_fold(0_i64, |sum, line| {
            line.trim()
                .parse::<i64>()
                .map(|count| sum + count)
                .map_err(|err| TestError::Command(err.to_string()))
        })
}
