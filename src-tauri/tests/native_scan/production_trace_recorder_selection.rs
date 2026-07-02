use std::{cell::RefCell, fs};

use morrow_diagnostics::diagnostics_trace_dir;
use morrow_lib::native_bridge::{
    CodexAuthStatus, CodexCommandOutput, CodexExecRequest, CodexExecRun, CodexExecRunner,
    CodexProviderAuthReadiness, NativeBridgeState, ProductionScanCodexDependencies,
};
use serde_json::json;

use super::{
    dependencies::RecordingProposalAdapter,
    message_sqlite::create_messages_fixture,
    support::{assert_counts, chat, scan_request},
};

#[derive(Debug)]
struct WriteOutputRunner {
    output: RefCell<Option<String>>,
}

impl WriteOutputRunner {
    fn new(output: String) -> Self {
        Self {
            output: RefCell::new(Some(output)),
        }
    }
}

impl CodexExecRunner for WriteOutputRunner {
    fn run_exec(&self, request: &CodexExecRequest) -> CodexExecRun {
        let Some(output) = self.output.borrow_mut().take() else {
            return CodexExecRun::FailedToStart;
        };
        match fs::write(request.output_path(), output) {
            Ok(()) => CodexExecRun::Completed(CodexCommandOutput::new(Some(0), "", "")),
            Err(error) => {
                CodexExecRun::Completed(CodexCommandOutput::new(Some(1), "", &error.to_string()))
            }
        }
    }
}

#[test]
fn production_trace_recorder_selection_disabled_uses_noop() -> Result<(), String> {
    // Given
    let fixture = ProductionTraceFixture::new("disabled")?;
    let request = fixture.scan_request(false)?;
    let runner = WriteOutputRunner::new(provider_candidate_json());
    let adapter = RecordingProposalAdapter::default();

    // When
    let result = fixture.scan(request, &runner, &adapter)?;

    // Then
    assert_counts(&result, (1, 1, 0, 1, 0));
    let traces_dir = diagnostics_trace_dir(&fixture.app_data_dir);
    if traces_dir.exists() {
        let trace_count = fs::read_dir(&traces_dir)
            .map_err(|error| error.to_string())?
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "jsonl")
            })
            .count();
        if trace_count != 0 {
            return Err(format!(
                "disabled diagnostics wrote {trace_count} trace files under {}",
                traces_dir.display()
            ));
        }
    }
    Ok(())
}

#[test]
fn production_trace_recorder_selection_enabled_writes_app_data_trace() -> Result<(), String> {
    // Given
    let fixture = ProductionTraceFixture::new("enabled")?;
    let request = fixture.scan_request(true)?;
    let runner = WriteOutputRunner::new(provider_candidate_json());
    let adapter = RecordingProposalAdapter::default();

    // When
    let result = fixture.scan(request, &runner, &adapter)?;

    // Then
    assert_counts(&result, (1, 1, 0, 1, 0));
    let traces_dir = diagnostics_trace_dir(&fixture.app_data_dir);
    if !traces_dir.starts_with(&fixture.app_data_dir) {
        return Err(format!(
            "trace dir {} is not rooted in app data {}",
            traces_dir.display(),
            fixture.app_data_dir.display()
        ));
    }
    let trace_files = trace_jsonl_files(&traces_dir)?;
    if trace_files.is_empty() {
        return Err(format!(
            "enabled diagnostics wrote no traces in {}",
            traces_dir.display()
        ));
    }
    let serialized = trace_files
        .iter()
        .map(fs::read_to_string)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?
        .join("\n");
    if !serialized.contains("\"component\":\"provider\"") {
        return Err(format!(
            "trace files did not include provider trace: {serialized}"
        ));
    }
    Ok(())
}

#[test]
fn production_trace_recorder_selection_enabled_sink_failure_is_nonfatal() -> Result<(), String> {
    // Given
    let fixture = ProductionTraceFixture::new("sink-failure")?;
    fs::write(&fixture.app_data_dir, "not a directory").map_err(|error| error.to_string())?;
    let request = fixture.scan_request(true)?;
    let runner = WriteOutputRunner::new(provider_candidate_json());
    let adapter = RecordingProposalAdapter::default();

    // When
    let result = fixture.scan(request, &runner, &adapter)?;

    // Then
    assert_counts(&result, (1, 1, 0, 1, 0));
    if !fixture.app_data_dir.is_file() {
        return Err("sink failure fixture app data path changed unexpectedly".to_owned());
    }
    Ok(())
}

struct ProductionTraceFixture {
    _dir: tempfile::TempDir,
    store_path: std::path::PathBuf,
    messages_db_path: std::path::PathBuf,
    app_data_dir: std::path::PathBuf,
}

impl ProductionTraceFixture {
    fn new(name: &str) -> Result<Self, String> {
        let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let store_path = dir.path().join(format!("{name}-morrow.sqlite"));
        let messages_db_path = dir.path().join(format!("{name}-chat.db"));
        let app_data_dir = dir.path().join("app-data");
        create_messages_fixture(&messages_db_path)?;
        Ok(Self {
            _dir: dir,
            store_path,
            messages_db_path,
            app_data_dir,
        })
    }

    fn scan_request(
        &self,
        local_diagnostics_enabled: bool,
    ) -> Result<morrow_lib::native_bridge::ScanSelectedChatsRequest, String> {
        let mut request = scan_request(
            &[chat(
                "messages-chat-8b96e568c027a42d3b5c9e6e7710201f",
                1,
                &["messages-participant-6044b729eea9fa126e78d421e4a41ac8"],
            )],
            &[],
            true,
            1,
            0,
        )?;
        request.local_diagnostics_enabled = local_diagnostics_enabled;
        request.local_diagnostics_retention_days = 30;
        Ok(request)
    }

    fn scan(
        &self,
        request: morrow_lib::native_bridge::ScanSelectedChatsRequest,
        runner: &WriteOutputRunner,
        adapter: &RecordingProposalAdapter,
    ) -> Result<morrow_lib::native_bridge::ScanSelectedChatsResult, String> {
        NativeBridgeState::default()
            .scan_selected_chats_at_with_codex_dependencies_and_app_data_dir(
                request,
                &self.store_path,
                &self.messages_db_path,
                &self.app_data_dir,
                ProductionScanCodexDependencies {
                    auth_readiness: logged_in_auth_readiness(),
                    codex_runner: runner,
                    proposal_adapter: adapter,
                },
            )
            .map_err(|error| error.to_string())
    }
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

fn trace_jsonl_files(traces_dir: &std::path::Path) -> Result<Vec<std::path::PathBuf>, String> {
    if !traces_dir.exists() {
        return Ok(Vec::new());
    }
    let mut paths = fs::read_dir(traces_dir)
        .map_err(|error| error.to_string())?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    paths.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "jsonl")
    });
    paths.sort();
    Ok(paths)
}
