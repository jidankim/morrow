use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use morrow_reconcile::feedback_eval::FeedbackEvalReport;
use morrow_storage::{FeedbackEvalCounts, FEEDBACK_EVAL_SCHEMA_VERSION};
use serde::Serialize;

use super::out_dir_policy::evidence_root;

pub(super) const RUN_MARKER: &str = "phase5-local-runner-v1";
pub(super) const REJECTION_MARKER: &str = "live-surface-rejection-v1";

type ArtifactResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug, Serialize)]
pub(crate) struct LocalTrajectoryArtifact {
    pub(crate) run_marker: &'static str,
    pub(crate) schema_version: i64,
    #[serde(skip)]
    pub(crate) artifact_path: PathBuf,
    pub(crate) case_results: Vec<CaseRunResult>,
    pub(crate) feedback_eval: FeedbackEvalSummary,
    pub(crate) storage_readback: StorageReadback,
    pub(crate) live_surface_proof: LiveSurfaceProof,
    pub(crate) cleanup_proof: CleanupProof,
    #[serde(skip)]
    pub(crate) serialized: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct CaseRunResult {
    pub(crate) trajectory_case_id: String,
    pub(crate) expected_outcomes: Vec<String>,
    pub(crate) expected_labels: Vec<String>,
    pub(crate) trace_operations: Vec<String>,
    pub(crate) local_fake_scan: bool,
    pub(crate) proposal_lifecycle: bool,
    pub(crate) lifecycle_replay: bool,
    pub(crate) phase4_approval_correction: String,
    pub(crate) diagnostics_surface: bool,
    pub(crate) decision_evidence_surface: bool,
    pub(crate) cleanup_action: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct FeedbackEvalSummary {
    pub(crate) status: String,
    pub(crate) cases_evaluated: i64,
    pub(crate) cases_skipped: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct StorageReadback {
    pub(crate) label_count: i64,
    pub(crate) feature_snapshot_count: i64,
    pub(crate) decision_evidence_count: usize,
    pub(crate) latest_eval_status: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct LiveSurfaceProof {
    pub(crate) no_live_surfaces_used: bool,
    pub(crate) forbidden_surfaces: [&'static str; 9],
}

#[derive(Debug, Serialize)]
pub(crate) struct CleanupProof {
    pub(crate) cleaned: bool,
    pub(crate) removed_paths: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct LiveSurfaceRejection {
    pub(crate) run_marker: &'static str,
    #[serde(skip)]
    pub(crate) artifact_path: PathBuf,
    pub(crate) rejected: bool,
    pub(crate) attempted: bool,
    pub(crate) boundary: &'static str,
    pub(crate) surface: &'static str,
    pub(crate) sanitized_target: &'static str,
    pub(crate) blocked_before_side_effect: bool,
    pub(crate) reason: &'static str,
    #[serde(skip)]
    pub(crate) serialized: String,
}

#[derive(Debug)]
pub(super) struct RunnerPaths {
    pub(super) db_path: PathBuf,
    pub(super) feedback_report_path: PathBuf,
}

impl RunnerPaths {
    pub(super) fn new(name: &str) -> Self {
        let root = std::env::temp_dir();
        let run_id = unique_run_id();
        Self {
            db_path: root.join(format!("morrow-phase5-{name}-{run_id}.sqlite3")),
            feedback_report_path: root.join(format!("morrow-phase5-{name}-{run_id}-feedback.json")),
        }
    }

    pub(super) fn cleanup_paths(&self) -> Vec<PathBuf> {
        vec![
            self.db_path.clone(),
            self.feedback_report_path.clone(),
            self.db_path.with_extension("sqlite3-wal"),
            self.db_path.with_extension("sqlite3-shm"),
            self.db_path.with_extension("sqlite3-journal"),
        ]
    }
}

pub(super) fn feedback_summary(report: &FeedbackEvalReport) -> FeedbackEvalSummary {
    FeedbackEvalSummary {
        status: report.status.clone(),
        cases_evaluated: report.cases_evaluated,
        cases_skipped: report.cases_skipped,
    }
}

pub(super) fn storage_readback(
    counts: FeedbackEvalCounts,
    decision_evidence_count: usize,
) -> StorageReadback {
    StorageReadback {
        label_count: counts.label_count,
        feature_snapshot_count: counts.feature_snapshot_count,
        decision_evidence_count,
        latest_eval_status: counts
            .latest_eval_status
            .map(|status| status.as_str().to_owned()),
    }
}

pub(super) const fn live_surface_proof() -> LiveSurfaceProof {
    LiveSurfaceProof {
        no_live_surfaces_used: true,
        forbidden_surfaces: [
            "codex",
            "openai",
            "provider_network",
            "messages_sqlite",
            "eventkit",
            "calendar",
            "reminders",
            "phoenix",
            "langfuse",
        ],
    }
}

pub(super) fn cleanup(paths: Vec<PathBuf>) -> ArtifactResult<CleanupProof> {
    let mut removed_paths = Vec::with_capacity(paths.len());
    for path in paths {
        reset_path(&path)?;
        removed_paths.push(
            path.file_name()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("phase5-local-runner-temp-artifact")
                .to_owned(),
        );
    }
    Ok(CleanupProof {
        cleaned: true,
        removed_paths,
    })
}

pub(super) fn reset_path(path: &Path) -> ArtifactResult<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Box::new(error)),
    }
}

pub(super) fn evidence_path(name: &str) -> ArtifactResult<PathBuf> {
    let root = evidence_root()?;
    fs::create_dir_all(&root)?;
    Ok(root.join(name))
}

pub(super) fn sanitize_evidence_text(input: &str) -> String {
    let mut sanitized = String::with_capacity(input.len());
    let mut remaining = input;
    while let Some((prefix_start, private_prefix)) = next_private_path(remaining) {
        sanitized.push_str(&remaining[..prefix_start]);
        sanitized.push_str("[REDACTED_LOCAL_PATH]");
        let skip_start = prefix_start + private_prefix.len();
        let skip_len = remaining[skip_start..]
            .find(is_path_boundary)
            .unwrap_or(remaining.len() - skip_start);
        remaining = &remaining[skip_start + skip_len..];
    }
    sanitized.push_str(remaining);
    sanitized
}

fn unique_run_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!("{}-{nanos}", std::process::id())
}

fn next_private_path(input: &str) -> Option<(usize, &'static str)> {
    let users = input.find("/Users/");
    let private = input.find("/private/");
    match (users, private) {
        (Some(user_index), Some(private_index)) if user_index <= private_index => {
            Some((user_index, "/Users/"))
        }
        (Some(_), Some(private_index)) => Some((private_index, "/private/")),
        (Some(user_index), None) => Some((user_index, "/Users/")),
        (None, Some(private_index)) => Some((private_index, "/private/")),
        (None, None) => None,
    }
}

fn is_path_boundary(character: char) -> bool {
    matches!(
        character,
        '"' | '\'' | '`' | ',' | ')' | ']' | '}' | '\n' | '\r' | '\t' | ' '
    )
}

const _: () = assert!(FEEDBACK_EVAL_SCHEMA_VERSION == 1);
