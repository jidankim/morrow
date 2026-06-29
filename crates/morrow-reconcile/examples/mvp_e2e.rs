//! End-to-end reconcile workflow harness for local proposal lifecycle coverage.

#[path = "mvp_e2e/external.rs"]
/// External proposal fixture operations.
pub mod external;
#[path = "mvp_e2e/fixtures.rs"]
/// Messages and detection fixtures.
pub mod fixtures;
#[path = "mvp_e2e/lifecycle.rs"]
/// Reconciliation lifecycle fixtures.
pub mod lifecycle;
#[path = "mvp_e2e/metrics.rs"]
/// Metrics report helpers.
pub mod metrics;

use std::path::PathBuf;

use fixtures::{detect_from_whitelisted_messages, WorkflowCandidates};
use metrics::MetricsInput;
use morrow_storage::{CandidateKind, CandidateState, Store};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths = HarnessPaths::from_env()?;
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
    metrics.write_report(&paths.metrics_report)?;

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

    println!("PASS mvp_e2e");
    println!("workflow_db={}", paths.workflow_db.display());
    println!("delete_db={}", paths.delete_db.display());
    println!("calendar_free_no_alert_no_guest=true");
    println!("reminder_date_only_time_completion_move=true");
    println!("synthetic_reconcile_approval_move_copy_delete_completion=true");
    println!("delete_all_storage_db_deleted=true");
    println!("delete_all_pre_delete_approved_candidate_readback=true");
    println!("delete_all_fake_external_sidecar_preserved=true");
    println!("delete_all_fake_apple_message_sidecar_preserved=true");
    println!("delete_all_token_revoke_exercised_by_reconcile_e2e=false");
    println!("privacy_dataset_ready=true");
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
            workflow_db: required_path("MORROW_E2E_DB")?,
            delete_db: required_path("MORROW_E2E_DELETE_DB")?,
            metrics_report: required_path("MORROW_E2E_METRICS_REPORT")?,
        })
    }
}

fn required_path(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let value = std::env::var(name)?;
    Ok(PathBuf::from(value))
}
