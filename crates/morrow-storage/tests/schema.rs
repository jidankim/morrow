use std::collections::BTreeSet;

use morrow_storage::Store;

mod support;

use support::{assert_feedback_eval_schema, sqlite_rows};

#[test]
fn migrations_create_required_tables_when_opening_fresh_database() {
    // Given: a fresh SQLite path.
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("schema.sqlite");

    // When: storage opens and runs migrations.
    let store = Store::open(&db_path).expect("open store");
    let tables = store.table_names().expect("table names");

    // Then: every MVP ledger table exists.
    let table_set = tables.into_iter().collect::<BTreeSet<_>>();
    let required = BTreeSet::from([
        "settings".to_owned(),
        "whitelist_chats".to_owned(),
        "candidates".to_owned(),
        "evidence".to_owned(),
        "audit_log".to_owned(),
        "suppressions".to_owned(),
        "quiet_logs".to_owned(),
        "external_object_mappings".to_owned(),
        "provider_model_prompt_versions".to_owned(),
        "replay_cursors".to_owned(),
        "feedback_events".to_owned(),
        "labels".to_owned(),
        "feature_snapshots".to_owned(),
        "eval_runs".to_owned(),
        "eval_results".to_owned(),
        "_morrow_migrations".to_owned(),
    ]);
    assert_eq!(table_set, required);
}

#[test]
fn migrations_record_versions_when_opening_fresh_database() {
    // Given: a fresh SQLite path.
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("migration-version.sqlite");

    // When: storage opens and runs migrations in order.
    let _store = Store::open(&db_path).expect("open store");
    let rows = sqlite_rows(
        &db_path,
        "SELECT version, name FROM _morrow_migrations ORDER BY version;",
    );

    // Then: each migration is recorded exactly once.
    assert_eq!(
        rows,
        vec![
            vec!["1".to_owned(), "init".to_owned()],
            vec!["2".to_owned(), "feedback_eval".to_owned()],
        ]
    );
}

#[test]
fn feedback_eval_schema_requires_new_tables() -> Result<(), String> {
    // Given: a fresh SQLite path opened through Store migrations.
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("feedback-eval-schema.sqlite");
    let _store = Store::open(&db_path).expect("open store");
    let reopened_store = Store::open(&db_path).expect("reopen store");

    // When: the schema is introspected from SQLite metadata.
    let table_set = reopened_store
        .table_names()
        .expect("table names")
        .into_iter()
        .collect::<BTreeSet<_>>();

    // Then: the feedback/eval tables, migration rows, constraints, and column guards hold.
    assert_feedback_eval_schema(&db_path, &table_set)
}
