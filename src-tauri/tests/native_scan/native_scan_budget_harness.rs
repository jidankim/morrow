use std::{
    env, fs,
    path::{Path, PathBuf},
};

use morrow_lib::native_bridge::{
    scan_selected_chats_with_dependencies, ScanSelectedChatsDependencies,
};

use super::{
    dependencies::{CandidateProvider, CountingProvider, RecordingProposalAdapter},
    message_sqlite::provider_route_outcome_count,
    provider_route_support::{provider_route_request, ProviderRouteFixture},
    support::{lock_storage_sqlite_run_log_env, run_current_test_in_child},
};

const STORAGE_SQLITE_RUN_LOG_ENV: &str = "MORROW_STORAGE_SQLITE_RUN_LOG";
const BUDGET_HARNESS_CHILD_ENV: &str = "MORROW_NATIVE_SCAN_BUDGET_HARNESS_CHILD";

#[test]
fn native_scan_budget_harness_counts_provider_sqlite_and_replay() -> Result<(), String> {
    if env::var_os(BUDGET_HARNESS_CHILD_ENV).is_none() {
        return run_current_test_in_child(
            "native_scan_budget_harness::native_scan_budget_harness_counts_provider_sqlite_and_replay",
            BUDGET_HARNESS_CHILD_ENV,
        );
    }

    // Given
    let _env_lock = lock_storage_sqlite_run_log_env();
    let fixture = ProviderRouteFixture::new()?;
    let run_log = fixture._dir.path().join("storage-sqlite-runs.log");
    let _guard = StorageSqliteRunLogGuard::set(run_log.clone());
    let request = provider_route_request()?;
    let provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    let result = scan_selected_chats_with_dependencies(
        request,
        &fixture.store_path,
        ScanSelectedChatsDependencies {
            source: &fixture.source,
            provider: &provider,
            proposal_adapter: &adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())?;

    // Then
    let provider_calls = provider.calls();
    let sqlite_runs = sqlite_run_count(&run_log)?;
    let provider_route_rows = provider_route_outcome_count(&fixture.store_path)?;
    let external_created = adapter.created_count();
    println!(
        "provider_calls={provider_calls} sqlite_runs={sqlite_runs} provider_route_rows={provider_route_rows} external_created={external_created}"
    );
    assert_eq!(provider_calls, 1);
    assert!(sqlite_runs > 0, "expected storage sqlite3 runs");
    assert_eq!(provider_route_rows, 1);
    assert_eq!(external_created, 1);
    assert_eq!(result.created_external_proposal_count, 1);
    Ok(())
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
