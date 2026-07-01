use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use schema_assertions::{
    assert_eval_result_columns, assert_eval_run_columns, assert_feedback_event_columns,
    assert_insert_fails, assert_label_columns, assert_snapshot_columns,
    assert_table_has_no_prohibited_columns, assert_unique_not_null_key,
};

mod schema_assertions;

const FIELD_SEPARATOR: &str = "\u{1f}";

pub fn sqlite_rows(db_path: &Path, sql: &str) -> Vec<Vec<String>> {
    let output = Command::new("sqlite3")
        .arg("-batch")
        .arg("-noheader")
        .arg("-separator")
        .arg(FIELD_SEPARATOR)
        .arg(db_path)
        .arg(sql)
        .output()
        .expect("run sqlite3");
    assert!(
        output.status.success(),
        "sqlite3 failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.split(FIELD_SEPARATOR).map(str::to_owned).collect())
        .collect()
}

pub fn assert_feedback_eval_schema(
    db_path: &Path,
    table_set: &BTreeSet<String>,
) -> Result<(), String> {
    let required_feedback_tables = BTreeSet::from([
        "feedback_events".to_owned(),
        "labels".to_owned(),
        "feature_snapshots".to_owned(),
        "eval_runs".to_owned(),
        "eval_results".to_owned(),
    ]);
    if !required_feedback_tables.is_subset(table_set) {
        return Err(format!(
            "missing feedback/eval tables: {:?}",
            required_feedback_tables
                .difference(table_set)
                .cloned()
                .collect::<Vec<_>>()
        ));
    }

    assert_eq!(
        sqlite_rows(
            db_path,
            "SELECT version, name FROM _morrow_migrations ORDER BY version;",
        ),
        vec![
            vec!["1".to_owned(), "init".to_owned()],
            vec!["2".to_owned(), "feedback_eval".to_owned()],
            vec!["3".to_owned(), "provider_route_outcomes".to_owned(),],
        ]
    );

    for table in [
        "feedback_events",
        "labels",
        "feature_snapshots",
        "eval_runs",
        "eval_results",
    ] {
        assert_table_has_no_prohibited_columns(db_path, table);
    }

    assert_feedback_event_columns(db_path);
    assert_label_columns(db_path);
    assert_snapshot_columns(db_path);
    assert_eval_run_columns(db_path);
    assert_eval_result_columns(db_path);

    assert_unique_not_null_key(db_path, "feedback_events", "event_key");
    assert_unique_not_null_key(db_path, "labels", "label_key");
    assert_unique_not_null_key(db_path, "feature_snapshots", "snapshot_key");
    assert_unique_not_null_key(db_path, "eval_runs", "run_key");
    assert_unique_not_null_key(db_path, "eval_results", "result_key");

    assert_insert_fails(
        db_path,
        "INSERT INTO feedback_events
         (event_key, event_type, schema_version, subject_type, subject_id, chat_guid,
          anchor_message_guid, source_excerpt_policy, label_source, privacy_tier, created_at)
         VALUES ('bad-event', 'not_allowed', 1, 'candidate', 'subject-1', 'chat-1',
                 'message-1', 'hide', 'lifecycle', 'internal_metadata', 1);",
    );
    assert_insert_fails(
        db_path,
        "INSERT INTO labels
         (label_key, label_type, label_value, schema_version, subject_type, subject_id,
          chat_guid, anchor_message_guid, source_excerpt_policy, label_source,
          privacy_tier, created_at)
         VALUES ('bad-label', 'proposal_outcome', 'ok', 1, 'candidate', 'subject-1',
                 'chat-1', 'message-1', 'hide', 'lifecycle', 'internal_metadata', 1);",
    );
    assert_insert_fails(
        db_path,
        "INSERT INTO eval_runs
         (run_key, status, schema_version, started_at, cases_evaluated, cases_skipped,
          approval_kept_rate_millis, observed_rejection_rate_millis, quiet_log_count,
          provider_failure_count, accepted_visible_ratio_millis, confusion_counts_json)
         VALUES ('bad-run', 'unknown', 1, 1, 0, 0, 0, 0, 0, 0, 0, '{}');",
    );
    assert_insert_fails(
        db_path,
        "INSERT INTO eval_results
         (result_key, eval_run_id, snapshot_key, label_key, expected_label_type,
          expected_label_value, outcome, created_at)
         VALUES ('bad-result', 1, 'snapshot-1', 'label-1', 'proposal_outcome',
                 'accepted', 'unknown', 1);",
    );
    Ok(())
}
