use std::{cell::RefCell, fs};

use morrow_lib::native_bridge::{
    CodexAuthStatus, CodexCommandOutput, CodexExecRequest, CodexExecRun, CodexExecRunner,
    CodexProviderAuthReadiness, NativeBridgeState, ProductionScanCodexDependencies,
};
use serde_json::json;

use super::dependencies::RecordingProposalAdapter;
use super::message_sqlite::create_messages_fixture;
use super::support::{assert_counts, chat, scan_request, scan_storage_dump};

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
fn scan_imperative_shell_preserves_selected_chat_contract() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let store_path = dir.path().join("morrow.sqlite");
    let messages_db_path = dir.path().join("chat.db");
    create_messages_fixture(&messages_db_path)?;
    let request = scan_request(
        &[chat(
            "messages-chat-8b96e568c027a42d3b5c9e6e7710201f",
            1,
            &["messages-participant-6044b729eea9fa126e78d421e4a41ac8"],
        )],
        &[],
        false,
        1,
        0,
    )?;
    let runner = WriteOutputRunner::new(
        json!({
            "kind": "calendar_event",
            "title": "Provider meeting",
            "confidence_millis": 800,
            "normalized_time": "2026-06-26T15:00:00[Asia/Seoul]",
            "anchor_evidence_id": "evidence://selected/0",
            "evidence_ids": ["evidence://selected/0"],
        })
        .to_string(),
    );
    let adapter = RecordingProposalAdapter::default();

    // When
    let result = NativeBridgeState::default()
        .scan_selected_chats_at_with_codex_dependencies(
            request,
            &store_path,
            &messages_db_path,
            ProductionScanCodexDependencies {
                auth_readiness: CodexProviderAuthReadiness {
                    status: CodexAuthStatus::LoggedInUsingChatGpt,
                    ready: true,
                    command_surface: "codex login status".to_owned(),
                    command_output_redacted: true,
                    diagnostic: "redacted test auth readiness".to_owned(),
                },
                codex_runner: &runner,
                proposal_adapter: &adapter,
            },
        )
        .map_err(|error| error.to_string())?;

    // Then
    assert_counts(&result, (1, 1, 0, 1, 0));
    assert_eq!(result.created_external_proposal_count, 1);
    let stored = scan_storage_dump(&store_path)?;
    if !stored.contains("messages-chat-8b96e568c027a42d3b5c9e6e7710201f") {
        return Err(format!("missing selected public chat id: {stored}"));
    }
    if stored.contains("iMessage;-;+15555550103") {
        return Err(format!("stored native chat id leaked: {stored}"));
    }
    Ok(())
}
