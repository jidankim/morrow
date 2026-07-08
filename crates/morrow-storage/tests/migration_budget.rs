use std::{ffi::OsString, fs, path::Path};

use morrow_storage::Store;

#[path = "support/mod.rs"]
mod support;

use crate::support::{lock_store_open_env, sqlite_rows};

const SQLITE_RUN_LOG_ENV: &str = "MORROW_STORAGE_SQLITE_RUN_LOG";

#[test]
fn migrations_use_batched_warm_open_budget() {
    // Given: a fresh SQLite path with sqlite3 process-run logging enabled.
    let _store_open_guard = lock_store_open_env();
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("batched-migration-budget.sqlite");
    let run_log_path = dir.path().join("sqlite-runs.log");
    let _env_guard = EnvVarGuard::set(SQLITE_RUN_LOG_ENV, run_log_path.as_os_str());

    // When: the database is opened from scratch.
    let _fresh_store = Store::open(&db_path).expect("fresh open store");
    let fresh_sqlite_runs = sqlite_run_count(&run_log_path);

    // Then: fresh migration bootstrap and all missing migrations are batched tightly.
    assert!(
        fresh_sqlite_runs <= 2,
        "fresh open used {fresh_sqlite_runs} sqlite3 runs"
    );
    assert_eq!(
        migration_version_counts(&db_path),
        expected_migration_counts()
    );

    // When: the same database is opened again.
    fs::write(&run_log_path, "").expect("reset sqlite run log");
    let _warm_store = Store::open(&db_path).expect("warm open store");
    let warm_sqlite_runs = sqlite_run_count(&run_log_path);

    // Then: warm open detects applied versions in one sqlite3 run, does not rerun migration 1,
    // and does not add duplicate migration rows.
    assert!(
        warm_sqlite_runs <= 1,
        "warm open used {warm_sqlite_runs} sqlite3 runs"
    );
    assert_eq!(
        migration_version_counts(&db_path),
        expected_migration_counts()
    );
}

fn migration_version_counts(db_path: &Path) -> Vec<Vec<String>> {
    sqlite_rows(
        db_path,
        "SELECT version, COUNT(*) FROM _morrow_migrations GROUP BY version ORDER BY version;",
    )
}

fn expected_migration_counts() -> Vec<Vec<String>> {
    (1..=9)
        .map(|version| vec![version.to_string(), "1".to_owned()])
        .collect()
}

fn sqlite_run_count(log_path: &Path) -> usize {
    fs::read_to_string(log_path)
        .map(|contents| contents.lines().count())
        .unwrap_or(0)
}

struct EnvVarGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &std::ffi::OsStr) -> Self {
        let previous = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}
