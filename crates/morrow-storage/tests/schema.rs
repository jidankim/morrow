use std::collections::BTreeSet;

use morrow_storage::Store;

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
        "_morrow_migrations".to_owned(),
    ]);
    assert_eq!(table_set, required);
}
