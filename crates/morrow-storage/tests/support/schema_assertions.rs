use std::path::Path;
use std::process::Command;

use super::sqlite_rows;

pub fn assert_feedback_event_columns(db_path: &Path) {
    assert_common_feedback_columns(
        db_path,
        "feedback_events",
        &["id", "event_key", "event_type"],
    );
}

pub fn assert_label_columns(db_path: &Path) {
    assert_common_feedback_columns(
        db_path,
        "labels",
        &["id", "label_key", "label_type", "label_value"],
    );
}

pub fn assert_snapshot_columns(db_path: &Path) {
    assert_common_feedback_columns(db_path, "feature_snapshots", &["id", "snapshot_key"]);
}

pub fn assert_eval_run_columns(db_path: &Path) {
    assert_columns_include(
        db_path,
        "eval_runs",
        &[
            "id",
            "run_key",
            "status",
            "schema_version",
            "started_at",
            "finished_at",
            "report_path",
            "cases_evaluated",
            "cases_skipped",
            "skip_reasons_json",
            "approval_kept_rate_millis",
            "observed_rejection_rate_millis",
            "quiet_log_count",
            "provider_failure_count",
            "accepted_visible_ratio_millis",
            "confusion_counts_json",
        ],
    );
}

pub fn assert_eval_result_columns(db_path: &Path) {
    assert_columns_include(
        db_path,
        "eval_results",
        &[
            "id",
            "result_key",
            "eval_run_id",
            "snapshot_key",
            "label_key",
            "expected_label_type",
            "expected_label_value",
            "actual_label_type",
            "actual_label_value",
            "outcome",
            "skip_reason",
            "created_at",
        ],
    );
}

pub fn assert_table_has_no_prohibited_columns(db_path: &Path, table: &str) {
    let column_names = column_names(db_path, table);
    for prohibited in [
        "embedding",
        "embeddings",
        "model_artifact",
        "full_prompt",
        "prompt_text",
        "full_response",
        "response_text",
        "raw_diagnostics_jsonl",
        "diagnostics_jsonl",
        "full_message_history",
        "message_history",
    ] {
        assert!(
            !column_names.iter().any(|column| column == prohibited),
            "{table} must not include prohibited column {prohibited}"
        );
    }
}

pub fn assert_unique_not_null_key(db_path: &Path, table: &str, key_column: &str) {
    let column_rows = sqlite_rows(db_path, &format!("PRAGMA table_info({table});"));
    let key_row = column_rows
        .iter()
        .find(|row| row.get(1).is_some_and(|column| column == key_column))
        .unwrap_or_else(|| panic!("{table}.{key_column} column missing"));
    assert_eq!(
        key_row.get(3).map(String::as_str),
        Some("1"),
        "{table}.{key_column} must be NOT NULL"
    );

    let index_rows = sqlite_rows(db_path, &format!("PRAGMA index_list({table});"));
    let unique_index_names = index_rows
        .into_iter()
        .filter(|row| row.get(2).is_some_and(|unique| unique == "1"))
        .filter_map(|row| row.get(1).cloned())
        .collect::<Vec<_>>();
    let has_unique_key_index = unique_index_names.iter().any(|index_name| {
        sqlite_rows(db_path, &format!("PRAGMA index_info({index_name});"))
            .into_iter()
            .any(|row| row.get(2).is_some_and(|column| column == key_column))
    });
    assert!(
        has_unique_key_index,
        "{table}.{key_column} must have a unique index"
    );
}

pub fn assert_insert_fails(db_path: &Path, sql: &str) {
    let output = Command::new("sqlite3")
        .arg("-batch")
        .arg(db_path)
        .arg(sql)
        .output()
        .expect("run sqlite3");
    assert!(
        !output.status.success(),
        "insert unexpectedly succeeded: {sql}"
    );
}

fn assert_common_feedback_columns(db_path: &Path, table: &str, table_columns: &[&str]) {
    let mut required = table_columns.to_vec();
    required.extend([
        "schema_version",
        "subject_type",
        "subject_id",
        "candidate_id",
        "chat_guid",
        "anchor_message_guid",
        "diagnostics_trace_id",
        "diagnostics_span_id",
        "diagnostics_parent_span_id",
        "diagnostics_chat_hash",
        "diagnostics_message_hash",
        "provider_model_prompt_version_id",
        "source_excerpt_policy",
        "label_source",
        "created_at",
        "expires_at",
        "privacy_tier",
        "privacy_metadata_json",
    ]);
    assert_columns_include(db_path, table, &required);
}

fn assert_columns_include(db_path: &Path, table: &str, required_columns: &[&str]) {
    let column_names = column_names(db_path, table);
    for required_column in required_columns {
        assert!(
            column_names.iter().any(|column| column == required_column),
            "{table}.{required_column} column missing"
        );
    }
}

fn column_names(db_path: &Path, table: &str) -> Vec<String> {
    sqlite_rows(db_path, &format!("PRAGMA table_info({table});"))
        .into_iter()
        .filter_map(|row| row.get(1).cloned())
        .collect::<Vec<_>>()
}
