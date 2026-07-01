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
        "provider_route_outcomes".to_owned(),
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
            vec!["3".to_owned(), "provider_route_outcomes".to_owned(),],
        ]
    );
}

#[test]
fn provider_route_schema_requires_version_three_table_and_constraints() {
    // Given: a fresh SQLite path opened through Store migrations.
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("provider-route-schema.sqlite");
    let _store = Store::open(&db_path).expect("open store");

    // When: the provider-route table metadata is inspected.
    let columns = sqlite_rows(&db_path, "PRAGMA table_info(provider_route_outcomes);");
    let column_names = columns
        .into_iter()
        .filter_map(|row| row.get(1).cloned())
        .collect::<BTreeSet<_>>();
    let duplicate = std::process::Command::new("sqlite3")
        .arg("-batch")
        .arg(&db_path)
        .arg(
            "INSERT INTO provider_route_outcomes
             (route_fingerprint, provider_route_contract_version,
              provider_candidate_schema_version, evidence_payload_hash, provider_id, model_id,
              prompt_version, source_excerpt_policy, reference_observed, reference_timezone,
              threshold_millis, parser_route, outcome_kind, candidate_kind, candidate_title,
              candidate_confidence_millis, candidate_normalized_time, candidate_evidence_excerpt,
              quiet_reason, created_at, updated_at)
             VALUES
             ('sha256:schema', 'contract', 'candidate-schema', 'sha256:evidence', 'provider',
              'model', 'prompt', 'include', '2026-07-01T09:00:00', 'Asia/Seoul', 700,
              'none', 'candidate', 'calendar_event', 'Messages event candidate', 900,
              '2026-07-02T18:00:00Z', 'dinner tomorrow at 6', NULL, 1, 1);
             INSERT INTO provider_route_outcomes
             (route_fingerprint, provider_route_contract_version,
              provider_candidate_schema_version, evidence_payload_hash, provider_id, model_id,
              prompt_version, source_excerpt_policy, reference_observed, reference_timezone,
              threshold_millis, parser_route, outcome_kind, quiet_reason, created_at, updated_at)
             VALUES
             ('sha256:schema', 'contract', 'candidate-schema', 'sha256:evidence', 'provider',
              'model', 'prompt', 'include', '2026-07-01T09:00:00', 'Asia/Seoul', 700,
              'none', 'quiet', 'stable quiet', 2, 2);",
        )
        .output()
        .expect("run sqlite3");
    let invalid_quiet = std::process::Command::new("sqlite3")
        .arg("-batch")
        .arg(&db_path)
        .arg(
            "INSERT INTO provider_route_outcomes
             (route_fingerprint, provider_route_contract_version,
              provider_candidate_schema_version, evidence_payload_hash, provider_id, model_id,
              prompt_version, source_excerpt_policy, reference_observed, reference_timezone,
              threshold_millis, parser_route, outcome_kind, created_at, updated_at)
             VALUES
             ('sha256:invalid-quiet', 'contract', 'candidate-schema', 'sha256:evidence',
              'provider', 'model', 'prompt', 'include', '2026-07-01T09:00:00', 'Asia/Seoul',
              700, 'none', 'quiet', 1, 1);",
        )
        .output()
        .expect("run sqlite3");

    // Then: required columns exist, fingerprints are unique, and quiet rows need a reason.
    for required in [
        "route_fingerprint",
        "provider_route_contract_version",
        "provider_candidate_schema_version",
        "evidence_payload_hash",
        "provider_id",
        "model_id",
        "prompt_version",
        "source_excerpt_policy",
        "reference_observed",
        "reference_timezone",
        "threshold_millis",
        "parser_route",
        "outcome_kind",
        "candidate_title",
        "quiet_reason",
        "created_at",
        "updated_at",
    ] {
        assert!(column_names.contains(required), "{required} column missing");
    }
    assert!(!duplicate.status.success());
    assert!(!invalid_quiet.status.success());
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
