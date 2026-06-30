//! Feedback eval replay command contract tests.

use std::path::Path;

use morrow_storage::{DetectionRouteLabel, FeedbackLabelValue, ProposalOutcomeLabel, Store};

#[path = "feedback_eval/support.rs"]
mod support;

use support::{
    assert_required_keys, create_symlink, read_report, reset_file, run_feedback_eval, run_mvp_e2e,
    seed_case, DB_PATH, PROVIDER_DB_PATH, PROVIDER_REPORT_PATH, REPORT_PATH, UNSAFE_DB_PATH,
    UNSAFE_REPORT_PATH,
};

#[test]
fn feedback_eval_report_schema() {
    // Given
    reset_file(DB_PATH);
    reset_file(REPORT_PATH);
    let store = Store::open(Path::new(DB_PATH)).expect("open seeded eval db");
    seed_case(
        &store,
        "accepted-subject",
        "deterministic_candidate",
        FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::Accepted),
    );

    // When
    let output = run_feedback_eval(DB_PATH, REPORT_PATH);

    // Then
    assert!(
        output.status.success(),
        "feedback_eval command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("feedback_eval_ready=true"));
    let report = read_report(REPORT_PATH);
    assert_required_keys(&report);
    assert_eq!(report["cases_evaluated"], 1);
    assert_eq!(report["cases_skipped"], 0);
    assert_eq!(store.eval_cases().expect("eval cases").len(), 1);
}

#[test]
fn feedback_eval_skips_live_provider_routes() {
    // Given
    reset_file(PROVIDER_DB_PATH);
    reset_file(PROVIDER_REPORT_PATH);
    let store = Store::open(Path::new(PROVIDER_DB_PATH)).expect("open provider eval db");
    seed_case(
        &store,
        "provider-subject",
        "provider_candidate",
        FeedbackLabelValue::DetectionRoute(DetectionRouteLabel::ProviderCandidate),
    );

    // When
    let output = run_feedback_eval(PROVIDER_DB_PATH, PROVIDER_REPORT_PATH);

    // Then
    assert!(
        output.status.success(),
        "feedback_eval command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("feedback_eval_ready=true"));
    assert!(stdout.contains("skipped_provider_network_disabled"));
    let report = read_report(PROVIDER_REPORT_PATH);
    assert_required_keys(&report);
    assert_eq!(report["cases_evaluated"], 0);
    assert_eq!(report["cases_skipped"], 1);
    assert_eq!(
        report["skip_reasons"]["skipped_provider_network_disabled"],
        1
    );
}

#[test]
fn feedback_eval_command_rejects_non_morrow_env_paths() {
    // Given
    reset_file(UNSAFE_DB_PATH);
    reset_file(UNSAFE_REPORT_PATH);

    // When
    let output = run_feedback_eval(UNSAFE_DB_PATH, UNSAFE_REPORT_PATH);

    // Then
    assert!(
        !output.status.success(),
        "non-Morrow paths must be rejected"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("MORROW_EVAL_DB"),
        "stderr should name the rejected env var"
    );
    assert!(!Path::new(UNSAFE_DB_PATH).exists());
    assert!(!Path::new(UNSAFE_REPORT_PATH).exists());
}

#[test]
fn feedback_eval_command_rejects_symlink_env_paths() {
    // Given
    let outside_db = Path::new("/tmp/not-morrow-feedback-eval-symlink-target.sqlite");
    let link_path = Path::new("/tmp/morrow-feedback-eval-link.sqlite");
    reset_file(outside_db.to_str().expect("utf8 outside path"));
    reset_file(link_path.to_str().expect("utf8 link path"));
    create_symlink(outside_db, link_path);

    // When
    let output = run_feedback_eval(
        link_path.to_str().expect("utf8 link path"),
        "/tmp/morrow-feedback-eval-link-report.json",
    );

    // Then
    assert!(!output.status.success(), "symlink db path must be rejected");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("MORROW_EVAL_DB"),
        "stderr should name the rejected env var"
    );
    assert!(!outside_db.exists());
    reset_file(link_path.to_str().expect("utf8 link path"));
    reset_file(outside_db.to_str().expect("utf8 outside path"));
    reset_file("/tmp/morrow-feedback-eval-link-report.json");
}

#[test]
fn feedback_eval_mvp_e2e_command_rejects_non_morrow_env_paths() {
    // Given
    let workflow_db = "/tmp/not-morrow-feedback-e2e.sqlite";
    let delete_db = "/tmp/not-morrow-feedback-e2e-delete.sqlite";
    let metrics_report = "/tmp/not-morrow-feedback-e2e-metrics.json";
    reset_file(workflow_db);
    reset_file(delete_db);
    reset_file(metrics_report);

    // When
    let output = run_mvp_e2e(workflow_db, delete_db, metrics_report);

    // Then
    assert!(
        !output.status.success(),
        "mvp_e2e non-Morrow paths must be rejected"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("MORROW_E2E_DB"),
        "stderr should name the rejected env var"
    );
    assert!(!Path::new(workflow_db).exists());
    assert!(!Path::new(delete_db).exists());
    assert!(!Path::new(metrics_report).exists());
}
