//! Local feedback eval replay reports.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use morrow_storage::{EvalResult, EvalRun, EvalRunStatus, FeedbackLabelValue, Store};
use serde::Serialize;

mod paths;
mod replay;

pub use paths::{synthetic_path_from_env, SyntheticPathError};

pub(crate) const EMPTY_DATASET: &str = "empty_dataset";
pub(crate) const SKIPPED_PROVIDER_NETWORK_DISABLED: &str = "skipped_provider_network_disabled";

/// Files used by a local feedback eval replay run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackEvalPaths {
    /// Local Morrow SQLite database path.
    pub db_path: PathBuf,
    /// JSON metrics report output path.
    pub report_path: PathBuf,
}

/// JSON metrics emitted by the local feedback eval replay command.
#[derive(Debug, Clone, Serialize)]
pub struct FeedbackEvalReport {
    /// Feedback/eval schema version.
    pub schema_version: i64,
    /// Database `eval_runs.id` value.
    pub eval_run_id: i64,
    /// Derived eval run status.
    pub status: String,
    /// Replayable cases evaluated.
    pub cases_evaluated: i64,
    /// Cases skipped because replay would require disabled behavior.
    pub cases_skipped: i64,
    /// Skip reason counts.
    pub skip_reasons: BTreeMap<String, i64>,
    /// Accepted proposal labels retained by current deterministic replay.
    pub approval_kept_rate_millis: i64,
    /// Observed rejection labels among evaluated cases.
    pub observed_rejection_rate_millis: i64,
    /// Quiet-log cases in the eval input.
    pub quiet_log_count: i64,
    /// Provider failure labels in the eval input.
    pub provider_failure_count: i64,
    /// Accepted labels whose snapshots still replay as visible proposals.
    pub accepted_visible_ratio_millis: i64,
    /// Expected-to-actual label count matrix.
    pub confusion_counts: BTreeMap<String, i64>,
}

/// Failure modes for local feedback eval replay.
#[derive(Debug)]
pub enum FeedbackEvalError {
    /// Storage open, read, or write failed.
    Storage(morrow_storage::StorageError),
    /// Report file I/O failed.
    ReportIo(std::io::Error),
    /// Report JSON serialization failed.
    ReportJson(serde_json::Error),
    /// System clock could not produce a run timestamp.
    Clock(std::time::SystemTimeError),
}

impl Display for FeedbackEvalError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(err) => write!(formatter, "feedback eval storage error: {err}"),
            Self::ReportIo(err) => write!(formatter, "feedback eval report I/O error: {err}"),
            Self::ReportJson(err) => write!(formatter, "feedback eval report JSON error: {err}"),
            Self::Clock(err) => write!(formatter, "feedback eval clock error: {err}"),
        }
    }
}

impl Error for FeedbackEvalError {}

/// Runs local DB-backed feedback eval replay and writes a JSON report.
pub fn run_feedback_eval(
    paths: &FeedbackEvalPaths,
) -> Result<FeedbackEvalReport, FeedbackEvalError> {
    let run_stamp = unix_run_stamp()?;
    let store = Store::open(&paths.db_path).map_err(FeedbackEvalError::Storage)?;
    let cases = match store.eval_cases() {
        Ok(cases) => cases,
        Err(_) => {
            return persist_failed_run(&store, &run_stamp, &paths.report_path, "malformed_input")
        }
    };
    let computed = replay::compute_report(&cases, &run_stamp, &paths.report_path);
    if let Err(err) = write_report(&paths.report_path, &computed.report) {
        let _ = persist_failed_run(
            &store,
            &run_stamp,
            &paths.report_path,
            "report_write_failed",
        );
        return Err(err);
    }
    let eval_run_id = store
        .create_eval_run(computed.run)
        .map_err(FeedbackEvalError::Storage)?;
    let report = FeedbackEvalReport {
        eval_run_id,
        ..computed.report
    };
    for result in computed.results {
        store
            .record_eval_result(EvalResult {
                eval_run_id,
                ..result
            })
            .map_err(FeedbackEvalError::Storage)?;
    }
    write_report(&paths.report_path, &report)?;
    Ok(report)
}

fn persist_failed_run(
    store: &Store,
    run_stamp: &RunStamp,
    report_path: &Path,
    reason: &str,
) -> Result<FeedbackEvalReport, FeedbackEvalError> {
    let report = failed_report(reason);
    let run = eval_run_from_report(
        format!("{}-{reason}", run_stamp.key(0)),
        EvalRunStatus::Failed,
        run_stamp,
        report_path,
        &report,
    );
    let eval_run_id = store
        .create_eval_run(run)
        .map_err(FeedbackEvalError::Storage)?;
    let report = FeedbackEvalReport {
        eval_run_id,
        ..report
    };
    write_report(report_path, &report)?;
    Ok(report)
}

fn failed_report(reason: &str) -> FeedbackEvalReport {
    let mut skip_reasons = BTreeMap::new();
    skip_reasons.insert(reason.to_owned(), 1);
    FeedbackEvalReport {
        schema_version: morrow_storage::FEEDBACK_EVAL_SCHEMA_VERSION,
        eval_run_id: 0,
        status: EvalRunStatus::Failed.as_str().to_owned(),
        cases_evaluated: 0,
        cases_skipped: 0,
        skip_reasons,
        approval_kept_rate_millis: 1_000,
        observed_rejection_rate_millis: 1_000,
        quiet_log_count: 0,
        provider_failure_count: 0,
        accepted_visible_ratio_millis: 1_000,
        confusion_counts: BTreeMap::new(),
    }
}

pub(crate) fn eval_run_from_report(
    run_key: String,
    status: EvalRunStatus,
    run_stamp: &RunStamp,
    report_path: &Path,
    report: &FeedbackEvalReport,
) -> EvalRun {
    EvalRun {
        id: None,
        run_key,
        status,
        schema_version: morrow_storage::FEEDBACK_EVAL_SCHEMA_VERSION,
        started_at: run_stamp.seconds,
        finished_at: Some(run_stamp.seconds + 1),
        report_path: Some(report_path.display().to_string()),
        cases_evaluated: report.cases_evaluated,
        cases_skipped: report.cases_skipped,
        skip_reasons_json: json_object(&report.skip_reasons),
        approval_kept_rate_millis: report.approval_kept_rate_millis,
        observed_rejection_rate_millis: report.observed_rejection_rate_millis,
        quiet_log_count: report.quiet_log_count,
        provider_failure_count: report.provider_failure_count,
        accepted_visible_ratio_millis: report.accepted_visible_ratio_millis,
        confusion_counts_json: json_object(&report.confusion_counts),
    }
}

#[derive(Debug)]
pub(crate) struct ComputedEval {
    pub(crate) report: FeedbackEvalReport,
    pub(crate) run: EvalRun,
    pub(crate) results: Vec<EvalResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReplayResult {
    pub(crate) actual: Option<FeedbackLabelValue>,
    pub(crate) outcome: morrow_storage::EvalResultOutcome,
    pub(crate) skip_reason: Option<String>,
}

#[derive(Debug)]
pub(crate) struct RunStamp {
    pub(crate) seconds: i64,
    nanos: u32,
}

impl RunStamp {
    pub(crate) fn key(&self, case_count: usize) -> String {
        format!("feedback-eval-{}-{}-{case_count}", self.seconds, self.nanos)
    }
}

fn write_report(path: &Path, report: &FeedbackEvalReport) -> Result<(), FeedbackEvalError> {
    let json = serde_json::to_string_pretty(report).map_err(FeedbackEvalError::ReportJson)?;
    fs::write(path, json).map_err(FeedbackEvalError::ReportIo)
}

pub(crate) fn json_object(values: &BTreeMap<String, i64>) -> String {
    serde_json::to_string(values).unwrap_or_else(|_| "{}".to_owned())
}

fn unix_run_stamp() -> Result<RunStamp, FeedbackEvalError> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(FeedbackEvalError::Clock)?;
    let seconds = i64::try_from(elapsed.as_secs()).map_err(|err| {
        FeedbackEvalError::ReportIo(std::io::Error::new(std::io::ErrorKind::InvalidData, err))
    })?;
    Ok(RunStamp {
        seconds,
        nanos: elapsed.subsec_nanos(),
    })
}
