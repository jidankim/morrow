use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use morrow_lib::native_bridge::{
    scan_selected_chats_with_dependencies, ScanSelectedChatsDependencies, ScanSelectedChatsError,
};
use morrow_storage::Store;

use super::dependencies::RecordingProposalAdapter;
use super::dependencies::{CandidateProvider, CountingProvider, InvalidJsonProvider};
use super::message_sqlite::provider_route_outcome_count;
use super::provider_route_ledger::{provider_route_request, ProviderRouteFixture};
use super::support::{
    assert_counts, lock_storage_sqlite_run_log_env, query_sqlite, run_current_test_in_child,
};

const STORAGE_SQLITE_RUN_LOG_ENV: &str = "MORROW_STORAGE_SQLITE_RUN_LOG";
const FEEDBACK_BATCHING_CHILD_ENV: &str = "MORROW_NATIVE_SCAN_FEEDBACK_BATCHING_CHILD";
const PRE_BATCH_CANDIDATE_SQLITE_RUNS: usize = 22;
const PRE_BATCH_QUIET_SQLITE_RUNS: usize = 14;

#[test]
fn scan_feedback_batching_keeps_provider_route_ledger_after_side_effects() -> Result<(), String> {
    if env::var_os(FEEDBACK_BATCHING_CHILD_ENV).is_none() {
        return run_current_test_in_child(
            "feedback_batching::scan_feedback_batching_keeps_provider_route_ledger_after_side_effects",
            FEEDBACK_BATCHING_CHILD_ENV,
        );
    }

    // Given: candidate feedback has the same observable rows as the existing feedback tests.
    let _env_lock = lock_storage_sqlite_run_log_env();
    let candidate = ProviderRouteFixture::with_text("Maybe meet tomorrow?")?;
    let candidate_run_log = fixture_path(&candidate)?.join("candidate-storage-sqlite-runs.log");
    let candidate_guard = StorageSqliteRunLogGuard::set(candidate_run_log.clone());
    let candidate_provider = CountingProvider::new(CandidateProvider);
    let candidate_adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    let candidate_result = scan_selected_chats_with_dependencies(
        provider_route_request()?,
        &candidate.store_path,
        ScanSelectedChatsDependencies {
            source: &candidate.source,
            provider: &candidate_provider,
            proposal_adapter: &candidate_adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())?;
    drop(candidate_guard);

    // Then
    assert_counts(&candidate_result, (1, 1, 0, 1, 0));
    assert_feedback_counts(&candidate.store_path, "1|1|1")?;
    assert_eq!(provider_route_outcome_count(&candidate.store_path)?, 1);
    let candidate_runs = sqlite_run_count(&candidate_run_log)?;
    println!("candidate_feedback_sqlite_runs={candidate_runs}");
    assert!(
        candidate_runs < PRE_BATCH_CANDIDATE_SQLITE_RUNS,
        "candidate feedback should persist snapshot/event/label in one sqlite3 run"
    );

    // Given: quiet feedback has the same observable rows as the existing feedback tests.
    let quiet = ProviderRouteFixture::with_text("Maybe meet tomorrow?")?;
    let quiet_run_log = fixture_path(&quiet)?.join("quiet-storage-sqlite-runs.log");
    let quiet_guard = StorageSqliteRunLogGuard::set(quiet_run_log.clone());
    let quiet_provider = CountingProvider::new(InvalidJsonProvider);
    let quiet_adapter = RecordingProposalAdapter::default();

    // When
    let quiet_result = scan_selected_chats_with_dependencies(
        provider_route_request()?,
        &quiet.store_path,
        ScanSelectedChatsDependencies {
            source: &quiet.source,
            provider: &quiet_provider,
            proposal_adapter: &quiet_adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())?;
    drop(quiet_guard);

    // Then
    assert_counts(&quiet_result, (0, 0, 1, 0, 0));
    assert_feedback_counts(&quiet.store_path, "1|1|2")?;
    assert_eq!(provider_route_outcome_count(&quiet.store_path)?, 1);
    let quiet_runs = sqlite_run_count(&quiet_run_log)?;
    println!("quiet_feedback_sqlite_runs={quiet_runs}");
    assert!(
        quiet_runs < PRE_BATCH_QUIET_SQLITE_RUNS,
        "quiet feedback should persist snapshot/event/all labels in one sqlite3 run"
    );

    // Given / When / Then: candidate feedback batch failure leaves no provider-route ledger row.
    let candidate_feedback_failure = ProviderRouteFixture::with_text("Maybe meet tomorrow?")?;
    Store::open(&candidate_feedback_failure.store_path).map_err(|error| error.to_string())?;
    install_feature_snapshot_failure_trigger(&candidate_feedback_failure.store_path)?;
    let failed_candidate_feedback = scan_selected_chats_with_dependencies(
        provider_route_request()?,
        &candidate_feedback_failure.store_path,
        ScanSelectedChatsDependencies {
            source: &candidate_feedback_failure.source,
            provider: &CountingProvider::new(CandidateProvider),
            proposal_adapter: &RecordingProposalAdapter::default(),
            trace_recorder: &recorder,
        },
    );
    assert_storage_error(failed_candidate_feedback);
    assert_eq!(
        provider_route_outcome_count(&candidate_feedback_failure.store_path)?,
        0
    );

    // Given / When / Then: quiet feedback batch failure leaves no provider-route ledger row.
    let quiet_feedback_failure = ProviderRouteFixture::with_text("Maybe meet tomorrow?")?;
    Store::open(&quiet_feedback_failure.store_path).map_err(|error| error.to_string())?;
    install_label_failure_trigger(&quiet_feedback_failure.store_path)?;
    let failed_quiet_feedback = scan_selected_chats_with_dependencies(
        provider_route_request()?,
        &quiet_feedback_failure.store_path,
        ScanSelectedChatsDependencies {
            source: &quiet_feedback_failure.source,
            provider: &CountingProvider::new(InvalidJsonProvider),
            proposal_adapter: &RecordingProposalAdapter::default(),
            trace_recorder: &recorder,
        },
    );
    assert_storage_error(failed_quiet_feedback);
    assert_eq!(
        provider_route_outcome_count(&quiet_feedback_failure.store_path)?,
        0
    );
    Ok(())
}

fn fixture_path(fixture: &ProviderRouteFixture) -> Result<PathBuf, String> {
    fixture
        .store_path
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "fixture store path has no parent".to_owned())
}

fn assert_feedback_counts(db_path: &Path, expected: &str) -> Result<(), String> {
    let counts = query_sqlite(
        db_path,
        "SELECT
            (SELECT COUNT(*) FROM feature_snapshots) || '|' ||
            (SELECT COUNT(*) FROM feedback_events) || '|' ||
            (SELECT COUNT(*) FROM labels);",
    )?;
    assert_eq!(counts.trim(), expected);
    Ok(())
}

fn assert_storage_error(
    result: Result<morrow_lib::native_bridge::ScanSelectedChatsResult, ScanSelectedChatsError>,
) {
    assert!(matches!(result, Err(ScanSelectedChatsError::Storage(_))));
}

fn install_feature_snapshot_failure_trigger(db_path: &Path) -> Result<(), String> {
    run_sqlite(
        db_path,
        "CREATE TRIGGER fail_feature_snapshot_insert BEFORE INSERT ON feature_snapshots BEGIN SELECT RAISE(FAIL, 'simulated feedback snapshot failure'); END;",
    )
}

fn install_label_failure_trigger(db_path: &Path) -> Result<(), String> {
    run_sqlite(
        db_path,
        "CREATE TRIGGER fail_label_insert BEFORE INSERT ON labels BEGIN SELECT RAISE(FAIL, 'simulated feedback label failure'); END;",
    )
}

fn run_sqlite(db_path: &Path, sql: &str) -> Result<(), String> {
    let output = Command::new("sqlite3")
        .arg(db_path)
        .arg(sql)
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

fn sqlite_run_count(path: &Path) -> Result<usize, String> {
    let contents = fs::read_to_string(path).map_err(|error| error.to_string())?;
    Ok(contents.lines().count())
}

struct StorageSqliteRunLogGuard {
    previous: Option<String>,
}

impl StorageSqliteRunLogGuard {
    fn set(path: PathBuf) -> Self {
        let previous = std::env::var(STORAGE_SQLITE_RUN_LOG_ENV).ok();
        std::env::set_var(STORAGE_SQLITE_RUN_LOG_ENV, path);
        Self { previous }
    }
}

impl Drop for StorageSqliteRunLogGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var(STORAGE_SQLITE_RUN_LOG_ENV, value),
            None => std::env::remove_var(STORAGE_SQLITE_RUN_LOG_ENV),
        }
    }
}
