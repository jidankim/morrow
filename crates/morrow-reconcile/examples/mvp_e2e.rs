//! End-to-end reconcile workflow harness for local proposal lifecycle coverage.

#[path = "mvp_e2e/external.rs"]
/// External proposal fixture operations.
pub mod external;
#[path = "mvp_e2e/feedback.rs"]
mod feedback;
#[path = "mvp_e2e/fixtures.rs"]
/// Messages and detection fixtures.
pub mod fixtures;
#[path = "mvp_e2e/lifecycle.rs"]
/// Reconciliation lifecycle fixtures.
pub mod lifecycle;
#[path = "mvp_e2e/metrics.rs"]
/// Metrics report helpers.
pub mod metrics;

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use fixtures::{detect_from_whitelisted_messages, WorkflowCandidates};
use metrics::{FeedbackEvalMetrics, MetricsInput};
use morrow_reconcile::feedback_eval::{
    run_feedback_eval, synthetic_path_from_env, FeedbackEvalPaths,
};
use morrow_storage::{CandidateKind, CandidateState, Store};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths = HarnessPaths::from_env()?;
    paths.reset_owned_artifacts()?;
    let store = Store::open(&paths.workflow_db)?;
    let WorkflowCandidates {
        calendar,
        reminder,
        quiet_logs,
        ignored_non_whitelisted,
    } = detect_from_whitelisted_messages(&store)?;

    let calendar_summary = external::create_calendar_proposal(&store, &calendar)?;
    let reminder_summary = external::create_reminder_proposals(&store, &reminder)?;
    let lifecycle_summary = lifecycle::run_lifecycle_cases(&store)?;
    let delete_summary = lifecycle::run_delete_all_case(&paths.delete_db)?;

    let metrics_input = MetricsInput {
        visible_proposals: calendar_summary.visible_created
            + reminder_summary.visible_created
            + lifecycle_summary.visible_proposals,
        approvals: lifecycle_summary.approvals,
        deletions: lifecycle_summary.deletions,
        false_positives: lifecycle_summary.deletions,
        latency_seconds_total: calendar_summary.latency_seconds
            + reminder_summary.latency_seconds
            + lifecycle_summary.latency_seconds,
        quiet_logs,
        user_days: 1,
    };
    let metrics = metrics::calculate(&metrics_input)?;
    let eval_report_path = paths.feedback_eval_report();
    let eval_report = run_feedback_eval(&FeedbackEvalPaths {
        db_path: paths.workflow_db.clone(),
        report_path: eval_report_path.clone(),
    })?;
    let feedback_counts = store.feedback_eval_counts()?;
    metrics.write_report(
        &paths.metrics_report,
        &FeedbackEvalMetrics {
            labels_recorded: feedback_counts.label_count,
            snapshots_recorded: feedback_counts.feature_snapshot_count,
            eval_results_recorded: eval_report.cases_evaluated + eval_report.cases_skipped,
            eval_report: eval_report_path.display().to_string(),
        },
    )?;

    assert_eq!(
        store.candidate_state(&calendar_summary.candidate_id)?,
        CandidateState::Visible
    );
    assert_eq!(
        store.candidate_state(&reminder_summary.candidate_id)?,
        CandidateState::Visible
    );
    assert_eq!(calendar.kind, CandidateKind::CalendarEvent);
    assert_eq!(reminder.kind, CandidateKind::TaskReminder);
    assert!(ignored_non_whitelisted);
    assert!(delete_summary.database_deleted);
    assert!(
        delete_summary
            .readbacks
            .approved_candidate_read_before_delete
    );
    assert!(
        delete_summary
            .readbacks
            .fake_approved_external_item_preserved
    );
    assert!(delete_summary.readbacks.fake_apple_message_source_preserved);
    assert!(
        delete_summary
            .readbacks
            .feedback_eval_case_read_before_delete
    );
    assert!(feedback_counts.label_count >= 3);
    assert!(feedback_counts.feature_snapshot_count >= 3);
    assert!(eval_report.cases_evaluated + eval_report.cases_skipped >= 3);
    assert!(eval_report_path.exists());

    println!("PASS mvp_e2e");
    println!("workflow_db={}", paths.workflow_db.display());
    println!("delete_db={}", paths.delete_db.display());
    println!("calendar_free_no_alert_no_guest=true");
    println!("reminder_date_only_time_completion_move=true");
    println!("synthetic_reconcile_approval_move_copy_delete_completion=true");
    println!("delete_all_storage_db_deleted=true");
    println!("delete_all_pre_delete_approved_candidate_readback=true");
    println!("delete_all_pre_delete_feedback_eval_case_readback=true");
    println!("delete_all_fake_external_sidecar_preserved=true");
    println!("delete_all_fake_apple_message_sidecar_preserved=true");
    println!("delete_all_token_revoke_exercised_by_reconcile_e2e=false");
    println!("privacy_dataset_ready=true");
    println!("feedback_eval_ready=true");
    println!("labels_recorded={}", feedback_counts.label_count);
    println!(
        "snapshots_recorded={}",
        feedback_counts.feature_snapshot_count
    );
    println!(
        "eval_results_recorded={}",
        eval_report.cases_evaluated + eval_report.cases_skipped
    );
    println!("eval_report={}", eval_report_path.display());
    println!(
        "metrics approval_rate_millis={} deletion_rate_millis={} false_positives={} \
         average_latency_seconds={} proposals_per_user_day={} quiet_log_candidates={}",
        metrics.approval_rate_millis,
        metrics.deletion_rate_millis,
        metrics.false_positives,
        metrics.average_latency_seconds,
        metrics.proposals_per_user_day,
        metrics.quiet_log_candidates
    );
    Ok(())
}

struct HarnessPaths {
    workflow_db: PathBuf,
    delete_db: PathBuf,
    metrics_report: PathBuf,
}

impl HarnessPaths {
    fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            workflow_db: synthetic_path_from_env("MORROW_E2E_DB")?,
            delete_db: synthetic_path_from_env("MORROW_E2E_DELETE_DB")?,
            metrics_report: synthetic_path_from_env("MORROW_E2E_METRICS_REPORT")?,
        })
    }

    fn feedback_eval_report(&self) -> PathBuf {
        self.metrics_report.with_extension("feedback-eval.json")
    }

    fn reset_owned_artifacts(&self) -> Result<(), Box<dyn std::error::Error>> {
        remove_if_exists(&self.workflow_db)?;
        remove_if_exists(&self.delete_db)?;
        remove_if_exists(&self.metrics_report)?;
        remove_if_exists(&self.feedback_eval_report())?;
        Ok(())
    }
}

fn remove_if_exists(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Box::new(error)),
    }
}
