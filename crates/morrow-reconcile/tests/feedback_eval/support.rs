use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

use morrow_storage::{
    DiagnosticsTraceLinkage, FeatureSnapshot, FeedbackLabelSource, FeedbackLabelValue,
    FeedbackPrivacyTier, FeedbackRecordMeta, FeedbackSourceExcerptPolicy, FeedbackSubjectType,
    Label, Store,
};
use serde_json::Value;

pub(super) const DB_PATH: &str = "/tmp/morrow-feedback-eval.sqlite";
pub(super) const REPORT_PATH: &str = "/tmp/morrow-feedback-eval-report.json";
pub(super) const PROVIDER_DB_PATH: &str = "/tmp/morrow-feedback-provider-eval.sqlite";
pub(super) const PROVIDER_REPORT_PATH: &str = "/tmp/morrow-feedback-provider-eval-report.json";
pub(super) const UNSAFE_DB_PATH: &str = "/tmp/not-morrow-feedback-eval.sqlite";
pub(super) const UNSAFE_REPORT_PATH: &str = "/tmp/not-morrow-feedback-eval-report.json";

static COMMAND_LOCK: Mutex<()> = Mutex::new(());

pub(super) fn run_feedback_eval(db_path: &str, report_path: &str) -> std::process::Output {
    let _guard = COMMAND_LOCK.lock().expect("feedback eval command lock");
    Command::new(env!("CARGO"))
        .args([
            "run",
            "--manifest-path",
            "crates/morrow-reconcile/Cargo.toml",
            "--example",
            "feedback_eval",
        ])
        .env("MORROW_EVAL_DB", db_path)
        .env("MORROW_EVAL_REPORT", report_path)
        .current_dir(workspace_root())
        .output()
        .expect("run feedback_eval example")
}

pub(super) fn run_mvp_e2e(
    workflow_db: &str,
    delete_db: &str,
    metrics_report: &str,
) -> std::process::Output {
    let _guard = COMMAND_LOCK.lock().expect("mvp e2e command lock");
    Command::new(env!("CARGO"))
        .args([
            "run",
            "--manifest-path",
            "crates/morrow-reconcile/Cargo.toml",
            "--example",
            "mvp_e2e",
        ])
        .env("MORROW_E2E_DB", workflow_db)
        .env("MORROW_E2E_DELETE_DB", delete_db)
        .env("MORROW_E2E_METRICS_REPORT", metrics_report)
        .current_dir(workspace_root())
        .output()
        .expect("run mvp_e2e example")
}

pub(super) fn seed_case(
    store: &Store,
    subject_id: &str,
    route: &str,
    label_value: FeedbackLabelValue,
) {
    let meta = meta(subject_id);
    store
        .record_feature_snapshot(FeatureSnapshot {
            id: None,
            snapshot_key: format!("snapshot-{subject_id}"),
            meta: meta.clone(),
            route: Some(route.to_owned()),
            reason_code: Some("test_fixture".to_owned()),
            confidence_millis: Some(900),
            participant_count: Some(2),
            tapback_signal: None,
            sender_signal_available: true,
            context_window_available: true,
            excerpt: None,
        })
        .expect("record feature snapshot");
    store
        .record_label(Label {
            id: None,
            label_key: format!("label-{subject_id}"),
            label_value,
            meta,
        })
        .expect("record label");
}

pub(super) fn assert_required_keys(report: &Value) {
    let object = report.as_object().expect("report is a JSON object");
    let keys = [
        "schema_version",
        "eval_run_id",
        "status",
        "cases_evaluated",
        "cases_skipped",
        "skip_reasons",
        "approval_kept_rate_millis",
        "observed_rejection_rate_millis",
        "quiet_log_count",
        "provider_failure_count",
        "accepted_visible_ratio_millis",
        "confusion_counts",
    ];
    for key in keys {
        assert!(object.contains_key(key), "missing report key {key}");
    }
    assert_eq!(object.len(), keys.len(), "unexpected report keys present");
}

pub(super) fn read_report(path: &str) -> Value {
    let raw = fs::read_to_string(path).expect("read feedback eval report");
    serde_json::from_str(&raw).expect("parse feedback eval report")
}

pub(super) fn reset_file(path: &str) {
    let _ = fs::remove_file(path);
}

pub(super) fn create_symlink(source: &Path, link: &Path) {
    unix_fs::symlink(source, link).expect("create db symlink");
}

fn meta(subject_id: &str) -> FeedbackRecordMeta {
    FeedbackRecordMeta {
        schema_version: 1,
        subject_type: FeedbackSubjectType::EvalCase,
        subject_id: subject_id.to_owned(),
        candidate_id: None,
        chat_guid: "feedback-eval-chat".to_owned(),
        anchor_message_guid: "feedback-eval-message".to_owned(),
        diagnostics: DiagnosticsTraceLinkage {
            trace_id: Some("trace_018fda8a98bf4cdba33a6f9d42180d6d".to_owned()),
            span_id: Some("span_52efdebab8574a5fa6f290831a786e86".to_owned()),
            parent_span_id: None,
            chat_hash: Some(
                "sha256:66e0bc3220b7dd3d0651965d244ebba7f5a8ae571be6874570b58495cdf26d85"
                    .to_owned(),
            ),
            message_hash: Some(
                "sha256:d9cee5362324ef2404962c149f0c564c7f6f009fe91ba5985cc5341a79a348de"
                    .to_owned(),
            ),
        },
        provider_model_prompt_version_id: None,
        source_excerpt_policy: FeedbackSourceExcerptPolicy::Hide,
        label_source: FeedbackLabelSource::ManualAlpha,
        privacy_tier: FeedbackPrivacyTier::InternalMetadata,
        privacy_metadata_json: "{}".to_owned(),
        created_at: 1_783_000_000,
        expires_at: None,
    }
}

fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
