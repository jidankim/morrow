use std::{cell::RefCell, fs, path::Path};

use morrow_calendar::ProposedEvent;
use morrow_lib::native_bridge::{
    CalendarProposalReceipt, CodexAuthStatus, CodexCommandOutput, CodexExecRequest, CodexExecRun,
    CodexExecRunner, CodexProviderAuthReadiness, NativeBridgeState,
    ProductionScanCodexDependencies, ProposalReplayAdapter, ScanSelectedChatsError,
    ScanSelectedChatsRequest, ScanSelectedChatsResult,
};
use morrow_storage::{EvalCase, ExternalObjectMapping, QueuedProposal, Store};
use serde_json::json;

mod messages_fixture;

use messages_fixture::{create_messages_fixture, create_messages_fixture_with_text};
pub use messages_fixture::{
    FIXTURE_MESSAGE_TEXT, NATIVE_CHAT_ID, NATIVE_MESSAGE_ID, PRIVACY_CANARY,
};

#[derive(Debug)]
pub struct RecordingCodexRunner {
    outcomes: RefCell<Vec<FakeCodexOutcome>>,
    run_count: RefCell<usize>,
}

impl RecordingCodexRunner {
    pub fn new(outcomes: Vec<FakeCodexOutcome>) -> Self {
        Self {
            outcomes: RefCell::new(outcomes),
            run_count: RefCell::new(0),
        }
    }

    pub fn run_count(&self) -> usize {
        *self.run_count.borrow()
    }
}

impl CodexExecRunner for RecordingCodexRunner {
    fn run_exec(&self, request: &CodexExecRequest) -> CodexExecRun {
        *self.run_count.borrow_mut() += 1;
        match self.outcomes.borrow_mut().pop() {
            Some(FakeCodexOutcome::WriteOutput(text)) => {
                match fs::write(request.output_path(), text) {
                    Ok(()) => CodexExecRun::Completed(CodexCommandOutput::new(Some(0), "", "")),
                    Err(error) => CodexExecRun::Completed(CodexCommandOutput::new(
                        Some(1),
                        "",
                        &error.to_string(),
                    )),
                }
            }
            Some(FakeCodexOutcome::TimedOut) => CodexExecRun::TimedOut,
            None => CodexExecRun::FailedToStart,
        }
    }
}

#[derive(Debug, Clone)]
pub enum FakeCodexOutcome {
    WriteOutput(String),
    TimedOut,
}

#[derive(Debug, Default)]
pub struct RejectingProposalAdapter;

impl ProposalReplayAdapter for RejectingProposalAdapter {
    fn create_calendar_proposal(
        &self,
        _event: ProposedEvent,
    ) -> Result<CalendarProposalReceipt, ScanSelectedChatsError> {
        Err(ScanSelectedChatsError::ExternalProposal(
            "calendar proposals are outside this scan provider test".to_owned(),
        ))
    }

    fn create_legacy_proposal(
        &self,
        _candidate: &QueuedProposal,
    ) -> Result<ExternalObjectMapping, ScanSelectedChatsError> {
        Err(ScanSelectedChatsError::ExternalProposal(
            "legacy proposals are outside this scan provider test".to_owned(),
        ))
    }
}

pub struct ScanFixture {
    _dir: tempfile::TempDir,
    store_path: std::path::PathBuf,
    messages_db_path: std::path::PathBuf,
    app_data_dir: std::path::PathBuf,
}

impl ScanFixture {
    pub fn new(name: &str) -> Result<Self, String> {
        Self::new_at(name, 1_782_352_400)
    }

    pub fn new_at(name: &str, message_unix_seconds: i64) -> Result<Self, String> {
        let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let store_path = dir.path().join(format!("{name}-morrow.sqlite"));
        let messages_db_path = dir.path().join(format!("{name}-chat.db"));
        let app_data_dir = dir.path().join("app-data");
        create_messages_fixture(&messages_db_path, message_unix_seconds)?;
        Ok(Self {
            _dir: dir,
            store_path,
            messages_db_path,
            app_data_dir,
        })
    }

    pub fn new_with_message_text(name: &str, message_text: &str) -> Result<Self, String> {
        let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let store_path = dir.path().join(format!("{name}-morrow.sqlite"));
        let messages_db_path = dir.path().join(format!("{name}-chat.db"));
        let app_data_dir = dir.path().join("app-data");
        create_messages_fixture_with_text(&messages_db_path, 1_782_352_400, message_text)?;
        Ok(Self {
            _dir: dir,
            store_path,
            messages_db_path,
            app_data_dir,
        })
    }

    pub fn app_data_dir(&self) -> &Path {
        &self.app_data_dir
    }

    pub fn store_path(&self) -> &Path {
        &self.store_path
    }

    pub fn eval_cases(&self) -> Result<Vec<EvalCase>, String> {
        Store::open(&self.store_path)
            .and_then(|store| store.eval_cases())
            .map_err(|error| error.to_string())
    }

    pub fn scan_with(
        &self,
        auth_readiness: CodexProviderAuthReadiness,
        runner: &RecordingCodexRunner,
        adapter: &RejectingProposalAdapter,
    ) -> Result<ScanSelectedChatsResult, String> {
        self.scan_with_request(scan_request()?, auth_readiness, runner, adapter)
    }

    pub fn scan_with_request(
        &self,
        request: ScanSelectedChatsRequest,
        auth_readiness: CodexProviderAuthReadiness,
        runner: &RecordingCodexRunner,
        adapter: &RejectingProposalAdapter,
    ) -> Result<ScanSelectedChatsResult, String> {
        NativeBridgeState::default()
            .scan_selected_chats_at_with_codex_dependencies(
                request,
                &self.store_path,
                &self.messages_db_path,
                ProductionScanCodexDependencies {
                    auth_readiness,
                    codex_runner: runner,
                    proposal_adapter: adapter,
                },
            )
            .map_err(|error| error.to_string())
    }

    pub fn scan_with_request_and_app_data_dir(
        &self,
        request: ScanSelectedChatsRequest,
        auth_readiness: CodexProviderAuthReadiness,
        runner: &RecordingCodexRunner,
        adapter: &RejectingProposalAdapter,
    ) -> Result<ScanSelectedChatsResult, String> {
        NativeBridgeState::default()
            .scan_selected_chats_at_with_codex_dependencies_and_app_data_dir(
                request,
                &self.store_path,
                &self.messages_db_path,
                &self.app_data_dir,
                ProductionScanCodexDependencies {
                    auth_readiness,
                    codex_runner: runner,
                    proposal_adapter: adapter,
                },
            )
            .map_err(|error| error.to_string())
    }
}

pub fn auth_readiness(status: CodexAuthStatus, ready: bool) -> CodexProviderAuthReadiness {
    CodexProviderAuthReadiness {
        status,
        ready,
        command_surface: "codex login status".to_owned(),
        command_output_redacted: true,
        diagnostic: "redacted test auth readiness".to_owned(),
    }
}

pub fn assert_counts(
    result: &ScanSelectedChatsResult,
    expected: (usize, usize, usize, usize, usize),
) {
    assert_eq!(
        (
            result.pending_proposal_count,
            result.created_candidate_count,
            result.quiet_log_count,
            result.cap_visible_count,
            result.cap_deferred_count,
        ),
        expected
    );
}

pub fn scan_result_counts(
    result: &ScanSelectedChatsResult,
) -> (usize, usize, usize, usize, usize, usize, usize) {
    (
        result.pending_proposal_count,
        result.created_candidate_count,
        result.quiet_log_count,
        result.cap_visible_count,
        result.cap_deferred_count,
        result.created_external_proposal_count,
        result.failed_external_proposal_count,
    )
}

fn scan_request() -> Result<ScanSelectedChatsRequest, String> {
    scan_request_at(1_782_352_400)
}

pub fn scan_request_at(reference_unix_seconds: i64) -> Result<ScanSelectedChatsRequest, String> {
    serde_json::from_value(json!({
        "selectedChatIds": ["messages-chat-8b96e568c027a42d3b5c9e6e7710201f"],
        "selectedChats": [
            {
                "id": "messages-chat-8b96e568c027a42d3b5c9e6e7710201f",
                "label": "Messages chat",
                "participantCount": 1,
                "participantIds": ["messages-participant-6044b729eea9fa126e78d421e4a41ac8"],
                "latestActivityTimestamp": reference_unix_seconds,
            },
        ],
        "referenceTimezone": "Asia/Seoul",
        "referenceUnixSeconds": reference_unix_seconds,
        "backfillPromptChatIds": [],
        "sourceExcerptsEnabled": true,
        "feedbackTextSnapshotsEnabled": false,
        "localDiagnosticsEnabled": false,
        "localDiagnosticsRetentionDays": 30,
        "capPolicy": {
            "mode": "refillForPending",
            "maxVisible": 0,
            "pendingCount": 0,
        },
    }))
    .map_err(|error| error.to_string())
}

pub fn candidate_json() -> String {
    candidate_json_at("2026-06-26T15:00:00[Asia/Seoul]")
}

pub fn candidate_json_at(normalized_time: &str) -> String {
    json!({
        "kind": "calendar_event",
        "title": "Provider meeting",
        "confidence_millis": 800,
        "normalized_time": normalized_time,
        "anchor_evidence_id": "evidence://selected/0",
        "evidence_ids": ["evidence://selected/0"],
    })
    .to_string()
}
