use std::error::Error;

use morrow_reconcile::feedback_eval::{run_feedback_eval, FeedbackEvalPaths};
use morrow_storage::{Store, FEEDBACK_EVAL_SCHEMA_VERSION};

mod artifact;
mod live_surface;
mod out_dir_policy;
mod surfaces;

pub(crate) use artifact::{LiveSurfaceRejection, LocalTrajectoryArtifact};

use artifact::{
    cleanup, evidence_path, feedback_summary, live_surface_proof, reset_path,
    sanitize_evidence_text, storage_readback, RunnerPaths, REJECTION_MARKER, RUN_MARKER,
};
use live_surface::{attempt_test_only_live_surface, LiveSurfaceAttempt};
use surfaces::run_case;

use super::fixtures::TRAJECTORY_CASES;
use super::validate_trajectory_cases;

type RunnerResult<T> = Result<T, Box<dyn Error>>;

pub(crate) fn run_local_trajectory_cases() -> RunnerResult<LocalTrajectoryArtifact> {
    validate_trajectory_cases(&TRAJECTORY_CASES)?;
    let paths = RunnerPaths::new("trajectory-run");
    reset_path(&paths.db_path)?;
    reset_path(&paths.feedback_report_path)?;
    let store = Store::open(&paths.db_path)?;
    let mut case_results = Vec::with_capacity(TRAJECTORY_CASES.len());
    for (index, case) in TRAJECTORY_CASES.iter().enumerate() {
        case_results.push(run_case(&store, case, index)?);
    }
    let feedback_report = run_feedback_eval(&FeedbackEvalPaths {
        db_path: paths.db_path.clone(),
        report_path: paths.feedback_report_path.clone(),
    })?;
    let counts = store.feedback_eval_counts()?;
    let decision_evidence_count = store
        .recent_decision_evidence(TRAJECTORY_CASES.len())?
        .len();
    let cleanup_proof = cleanup(paths.cleanup_paths())?;
    let artifact_path = evidence_path("local-trajectory-run.json")?;
    let mut artifact = LocalTrajectoryArtifact {
        run_marker: RUN_MARKER,
        schema_version: FEEDBACK_EVAL_SCHEMA_VERSION,
        artifact_path,
        case_results,
        feedback_eval: feedback_summary(&feedback_report),
        storage_readback: storage_readback(counts, decision_evidence_count),
        live_surface_proof: live_surface_proof(),
        cleanup_proof,
        serialized: String::new(),
    };
    artifact.serialized = sanitize_evidence_text(&serde_json::to_string_pretty(&artifact)?);
    std::fs::write(&artifact.artifact_path, &artifact.serialized)?;
    Ok(artifact)
}

pub(crate) fn run_local_trajectory_cases_with_live_attempt() -> RunnerResult<LiveSurfaceRejection> {
    let blocked = attempt_test_only_live_surface(LiveSurfaceAttempt::messages_sqlite_probe())
        .err()
        .ok_or_else(|| std::io::Error::other("test live surface probe unexpectedly succeeded"))?;
    let artifact_path = evidence_path("live-surface-rejection.json")?;
    let mut rejection = LiveSurfaceRejection {
        run_marker: REJECTION_MARKER,
        artifact_path,
        rejected: true,
        attempted: true,
        boundary: blocked.boundary,
        surface: blocked.surface,
        sanitized_target: blocked.sanitized_target,
        blocked_before_side_effect: true,
        reason: blocked.reason,
        serialized: String::new(),
    };
    rejection.serialized = sanitize_evidence_text(&serde_json::to_string_pretty(&rejection)?);
    std::fs::write(&rejection.artifact_path, &rejection.serialized)?;
    Ok(rejection)
}

#[cfg(test)]
pub(crate) fn with_out_dir_env_lock<T>(run: impl FnOnce() -> T) -> T {
    out_dir_policy::with_out_dir_env_lock(run)
}
