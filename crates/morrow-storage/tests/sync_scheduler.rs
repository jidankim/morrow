use std::path::Path;
use std::process::Command;

use morrow_storage::{
    Store, SyncSchedulerIntervalSeconds, SyncSchedulerLastResult, SyncSchedulerState,
    SyncSchedulerStatus,
};

const FIELD_SEPARATOR: &str = "\u{1f}";

fn fresh_store(name: &str) -> (tempfile::TempDir, std::path::PathBuf, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join(name);
    let store = Store::open(&db_path).expect("open store");
    (dir, db_path, store)
}

fn sqlite_rows(db_path: &Path, sql: &str) -> Vec<Vec<String>> {
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

fn assert_sqlite_rejects(db_path: &Path, sql: &str) {
    let output = Command::new("sqlite3")
        .arg("-batch")
        .arg(db_path)
        .arg(sql)
        .output()
        .expect("run sqlite3");
    assert!(
        !output.status.success(),
        "sqlite3 unexpectedly accepted malformed_input: {sql}"
    );
}

#[test]
fn sync_scheduler_default_row_is_created_when_opening_fresh_database() {
    // Given: a fresh storage database path.
    let (_dir, db_path, store) = fresh_store("sync-scheduler-default.sqlite");

    // When: the singleton scheduler row is loaded through the typed API.
    let state = store
        .load_sync_scheduler_state()
        .expect("load sync scheduler state");
    let rows = sqlite_rows(
        &db_path,
        "SELECT id, enabled, interval_seconds, status, retry_attempt
         FROM sync_scheduler_state;",
    );

    // Then: automatic sync is durably disabled by default with the 30 minute interval.
    assert_eq!(
        state,
        SyncSchedulerState {
            enabled: false,
            interval_seconds: SyncSchedulerIntervalSeconds::THIRTY_MINUTES,
            status: SyncSchedulerStatus::Disabled,
            last_started_at: None,
            last_finished_at: None,
            next_run_at: None,
            next_eligible_at: None,
            last_result: None,
            retry_attempt: 0,
            last_reason: None,
            updated_at: 0,
        }
    );
    assert_eq!(
        rows,
        vec![vec![
            "1".to_owned(),
            "0".to_owned(),
            "1800".to_owned(),
            "disabled".to_owned(),
            "0".to_owned(),
        ]]
    );
    println!("sync_scheduler_default_row={rows:?}");
}

#[test]
fn sync_scheduler_save_accepts_custom_whole_minute_interval_values() {
    // Given: a fresh scheduler row.
    let (_dir, _db_path, store) = fresh_store("sync-scheduler-intervals.sqlite");

    for interval_seconds in [
        SyncSchedulerIntervalSeconds::ONE_MINUTE,
        SyncSchedulerIntervalSeconds::THIRTY_MINUTES,
        SyncSchedulerIntervalSeconds::parse(420).expect("custom seven minute interval"),
    ] {
        // When: a state using a custom whole-minute scheduler interval is saved.
        store
            .save_sync_scheduler_state(&SyncSchedulerState {
                enabled: true,
                interval_seconds,
                status: SyncSchedulerStatus::Scheduled,
                last_started_at: None,
                last_finished_at: None,
                next_run_at: Some(1_783_000_000 + interval_seconds.as_i64()),
                next_eligible_at: None,
                last_result: None,
                retry_attempt: 0,
                last_reason: None,
                updated_at: 1_783_000_000,
            })
            .expect("save allowed interval state");

        // Then: the typed API returns the exact whole-minute interval.
        let persisted = store
            .load_sync_scheduler_state()
            .expect("reload interval state");
        assert_eq!(persisted.interval_seconds, interval_seconds);
        assert_eq!(persisted.status, SyncSchedulerStatus::Scheduled);
    }
}

#[test]
fn sync_scheduler_cooldown_fields_persist_after_reopen() {
    // Given: a fresh scheduler row and a cooldown state with every nullable field populated.
    let (_dir, db_path, store) = fresh_store("sync-scheduler-reopen.sqlite");
    let cooldown = SyncSchedulerState {
        enabled: true,
        interval_seconds: SyncSchedulerIntervalSeconds::FIFTEEN_MINUTES,
        status: SyncSchedulerStatus::Cooldown,
        last_started_at: Some(1_783_000_100),
        last_finished_at: Some(1_783_000_160),
        next_run_at: Some(1_783_001_060),
        next_eligible_at: Some(1_783_000_460),
        last_result: Some(SyncSchedulerLastResult::RetryableFailure),
        retry_attempt: 2,
        last_reason: Some("temporary provider failure".to_owned()),
        updated_at: 1_783_000_161,
    };

    // When: the state is saved and the database is reopened.
    store
        .save_sync_scheduler_state(&cooldown)
        .expect("save cooldown state");
    let reopened = Store::open(&db_path).expect("reopen store");
    let persisted = reopened
        .load_sync_scheduler_state()
        .expect("load reopened cooldown state");

    // Then: stale_state preserves every scheduler/backoff field exactly.
    assert_eq!(persisted, cooldown);
    println!("sync_scheduler_reopened_cooldown={persisted:?}");
}

#[test]
fn sync_scheduler_sqlite_constraints_reject_malformed_input() {
    // Given: a migrated scheduler table with the singleton row.
    let (_dir, db_path, _store) = fresh_store("sync-scheduler-constraints.sqlite");

    // When/Then: malformed_input writes rejected by SQLite include below-minimum intervals,
    // sub-minute intervals, and statuses.
    assert_sqlite_rejects(
        &db_path,
        "UPDATE sync_scheduler_state SET interval_seconds = 59 WHERE id = 1;",
    );
    assert_sqlite_rejects(
        &db_path,
        "UPDATE sync_scheduler_state SET interval_seconds = 61 WHERE id = 1;",
    );
    assert_sqlite_rejects(
        &db_path,
        "UPDATE sync_scheduler_state SET status = 'bogus' WHERE id = 1;",
    );
    assert_sqlite_rejects(
        &db_path,
        "INSERT INTO sync_scheduler_state
         (id, enabled, interval_seconds, status, retry_attempt, updated_at)
         VALUES (2, 0, 1800, 'disabled', 0, 0);",
    );
    println!("sync_scheduler_malformed_input_rejected=true");
}
