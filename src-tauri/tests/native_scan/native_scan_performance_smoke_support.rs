use std::{
    env, fs,
    path::{Path, PathBuf},
};

use morrow_lib::native_bridge::{
    CodexAuthStatus, CodexProviderAuthReadiness, MessagesDiscoveryCommandReport, NativeBridgeState,
    ProductionScanCodexDependencies, ScanSelectedChatsRequest,
};
use morrow_messages::MessagesDiscoveryStatus;
use morrow_storage::Store;
use serde_json::json;

use crate::{
    dependencies::RecordingProposalAdapter,
    message_sqlite::{create_messages_fixture, provider_route_outcome_count},
    support::{chat, scan_request},
};

#[path = "native_scan_performance_smoke_runner.rs"]
mod native_scan_performance_smoke_runner;

use native_scan_performance_smoke_runner::WriteOutputRunner;

pub(super) const STORAGE_SQLITE_RUN_LOG_ENV: &str = "MORROW_STORAGE_SQLITE_RUN_LOG";
pub(super) const SMOKE_CHILD_ENV: &str = "MORROW_NATIVE_SCAN_PERFORMANCE_SMOKE_CHILD";
pub(super) const BUDGET_LINE_PREFIX: &str = "PERF_NATIVE_SCAN_SMOKE provider_calls=";

pub(super) fn run_native_scan_performance_smoke_child() -> Result<(), String> {
    // Given
    let run_log = env::var_os(STORAGE_SQLITE_RUN_LOG_ENV)
        .map(PathBuf::from)
        .ok_or_else(|| format!("{STORAGE_SQLITE_RUN_LOG_ENV} was not set"))?;
    let fixture = ProductionSmokeFixture::new()?;
    let state = NativeBridgeState::default();
    let discovery = state
        .discover_messages_chats_at(&fixture.messages_db_path)
        .map_err(|error| error.to_string())?;
    if discovery.status() != MessagesDiscoveryStatus::Ready {
        return Err(format!("expected ready discovery, got {discovery:?}"));
    }
    let command_discovery = MessagesDiscoveryCommandReport::from_report(&discovery);
    let selected = SelectedPublicChat::from_discovery(&command_discovery)?;
    let runner = WriteOutputRunner::with_repeated_output(provider_candidate_json(), 2);
    let adapter = RecordingProposalAdapter::default();

    // When
    let first = state
        .scan_selected_chats_at_with_codex_dependencies(
            selected.request(1_782_352_400)?,
            &fixture.store_path,
            &fixture.messages_db_path,
            ProductionScanCodexDependencies {
                auth_readiness: logged_in_auth_readiness(),
                codex_runner: &runner,
                proposal_adapter: &adapter,
            },
        )
        .map_err(|error| error.to_string())?;
    let external_after_first = adapter.created_count();
    let same_day = state
        .scan_selected_chats_at_with_codex_dependencies(
            selected.request(1_782_359_580)?,
            &fixture.store_path,
            &fixture.messages_db_path,
            ProductionScanCodexDependencies {
                auth_readiness: logged_in_auth_readiness(),
                codex_runner: &runner,
                proposal_adapter: &adapter,
            },
        )
        .map_err(|error| error.to_string())?;
    let second_scan_created = adapter.created_count() - external_after_first;
    let provider_calls_after_same_day = runner.calls();
    state
        .scan_selected_chats_at_with_codex_dependencies(
            selected.request(1_782_402_000)?,
            &fixture.store_path,
            &fixture.messages_db_path,
            ProductionScanCodexDependencies {
                auth_readiness: logged_in_auth_readiness(),
                codex_runner: &runner,
                proposal_adapter: &adapter,
            },
        )
        .map_err(|error| error.to_string())?;
    let _store = Store::open(&fixture.store_path).map_err(|error| error.to_string())?;
    let budget = SmokeBudget {
        provider_calls: runner.calls(),
        sqlite_runs: sqlite_run_count(&run_log)?,
        provider_route_rows: provider_route_outcome_count(&fixture.store_path)?,
        external_created: adapter.created_count(),
        second_scan_created,
    };
    let budget_line = budget.to_line();
    println!("{budget_line}");

    // Then
    if first.created_external_proposal_count != 1 {
        return Err(format!(
            "first production scan should create one external proposal, got {}",
            first.created_external_proposal_count
        ));
    }
    if same_day.created_external_proposal_count != 0 {
        return Err(format!(
            "same-day cache hit should not create an external proposal, got {}",
            same_day.created_external_proposal_count
        ));
    }
    if runner.calls() != provider_calls_after_same_day + 1 {
        return Err(format!(
            "changed local date should refresh provider route once: {budget_line}"
        ));
    }
    if budget.provider_calls != 2 {
        return Err(format!(
            "provider calls should be first scan plus changed-local-date invalidation only: {budget_line}"
        ));
    }
    if budget.sqlite_runs == 0 {
        return Err(format!("expected storage sqlite runs: {budget_line}"));
    }
    if budget.provider_route_rows != 2 {
        return Err(format!(
            "expected two provider-route rows after changed-local-date invalidation: {budget_line}"
        ));
    }
    if budget.external_created != 1 {
        return Err(format!(
            "expected replay adapter to dedupe the already-created external proposal: {budget_line}"
        ));
    }
    if budget.second_scan_created != 0 {
        return Err(format!(
            "same-day cache reuse should create no second-scan external proposal: {budget_line}"
        ));
    }
    if !budget_line.starts_with(BUDGET_LINE_PREFIX) {
        return Err(format!("missing final budget prefix: {budget_line}"));
    }
    Ok(())
}

struct ProductionSmokeFixture {
    _dir: tempfile::TempDir,
    store_path: PathBuf,
    messages_db_path: PathBuf,
}

impl ProductionSmokeFixture {
    fn new() -> Result<Self, String> {
        let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let store_path = dir.path().join("morrow.sqlite");
        let messages_db_path = dir.path().join("chat.db");
        create_messages_fixture(&messages_db_path)?;
        Ok(Self {
            _dir: dir,
            store_path,
            messages_db_path,
        })
    }
}

struct SelectedPublicChat {
    chat_id: String,
    participant_id: String,
}

impl SelectedPublicChat {
    fn from_discovery(discovery: &MessagesDiscoveryCommandReport) -> Result<Self, String> {
        let chat = discovery
            .chats
            .first()
            .ok_or_else(|| "missing discovered chat".to_owned())?;
        let participant_id = chat
            .participant_ids
            .first()
            .cloned()
            .ok_or_else(|| "missing discovered participant".to_owned())?;
        Ok(Self {
            chat_id: chat.chat_id.clone(),
            participant_id,
        })
    }

    fn request(&self, reference_unix_seconds: i64) -> Result<ScanSelectedChatsRequest, String> {
        let mut request = scan_request(
            &[chat(
                self.chat_id.as_str(),
                1,
                &[self.participant_id.as_str()],
            )],
            &[],
            true,
            1,
            0,
        )?;
        request.reference_unix_seconds = Some(reference_unix_seconds);
        request.local_diagnostics_enabled = false;
        request.local_diagnostics_retention_days = 30;
        Ok(request)
    }
}

#[derive(Debug, Clone, Copy)]
struct SmokeBudget {
    provider_calls: usize,
    sqlite_runs: usize,
    provider_route_rows: i64,
    external_created: usize,
    second_scan_created: usize,
}

impl SmokeBudget {
    fn to_line(self) -> String {
        format!(
            "PERF_NATIVE_SCAN_SMOKE provider_calls={} sqlite_runs={} provider_route_rows={} external_created={} second_scan_created={}",
            self.provider_calls,
            self.sqlite_runs,
            self.provider_route_rows,
            self.external_created,
            self.second_scan_created
        )
    }
}

fn provider_candidate_json() -> String {
    json!({
        "kind": "calendar_event",
        "title": "Provider meeting",
        "confidence_millis": 800,
        "normalized_time": "2026-06-26T15:00:00[Asia/Seoul]",
        "anchor_evidence_id": "evidence://selected/0",
        "evidence_ids": ["evidence://selected/0"],
    })
    .to_string()
}

fn logged_in_auth_readiness() -> CodexProviderAuthReadiness {
    CodexProviderAuthReadiness {
        status: CodexAuthStatus::LoggedInUsingChatGpt,
        ready: true,
        command_surface: "codex login status".to_owned(),
        command_output_redacted: true,
        diagnostic: "redacted test auth readiness".to_owned(),
    }
}

fn sqlite_run_count(path: &Path) -> Result<usize, String> {
    let contents = fs::read_to_string(path).map_err(|error| error.to_string())?;
    Ok(contents.lines().count())
}
