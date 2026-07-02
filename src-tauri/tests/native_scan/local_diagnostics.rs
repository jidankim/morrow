use std::cell::Cell;

use morrow_lib::native_bridge::{
    CodexAuthStatus, CodexCommandOutput, CodexExecRequest, CodexExecRun, CodexExecRunner,
    CodexProviderAuthReadiness, NativeBridgeState, ProductionScanCodexDependencies,
};
use morrow_messages::NativeBatch;
use serde_json::json;

use super::{
    dependencies::RecordingProposalAdapter,
    support::{
        chat, fake_state, malformed_request, native_batch, scan_request, selected_chat_json,
        temp_db,
    },
};

#[derive(Debug, Default)]
struct CountingCodexRunner {
    calls: Cell<usize>,
}

impl CountingCodexRunner {
    fn calls(&self) -> usize {
        self.calls.get()
    }
}

impl CodexExecRunner for CountingCodexRunner {
    fn run_exec(&self, _request: &CodexExecRequest) -> CodexExecRun {
        self.calls.set(self.calls.get() + 1);
        CodexExecRun::Completed(CodexCommandOutput::new(
            Some(1),
            "",
            "unexpected provider call",
        ))
    }
}

#[test]
fn scan_selected_chats_rejects_malformed_selected_metadata() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-malformed.sqlite")?;
    let state = fake_state(&db_path, NativeBatch::default());
    let cases = [
        (
            "empty selected list",
            json!([]),
            json!([]),
            "selected_chats",
        ),
        (
            "mismatched selected ids",
            json!(["chat-a"]),
            json!([selected_chat_json("chat-b", 1, &["p1"])]),
            "selected_chats",
        ),
        (
            "duplicate participant ids",
            json!(["chat-a"]),
            json!([selected_chat_json("chat-a", 2, &["p1", "p1"])]),
            "participant_ids",
        ),
        (
            "invalid guid",
            json!([""]),
            json!([selected_chat_json("", 1, &["p1"])]),
            "chat_guid",
        ),
    ];
    for (name, selected_chat_ids, selected_chats, expected) in cases {
        // When
        let error = state
            .scan_selected_chats_at(
                malformed_request(selected_chat_ids, selected_chats)?,
                &db_path,
                &db_path,
            )
            .err()
            .ok_or_else(|| format!("{name}: scan unexpectedly succeeded"))?
            .to_string();
        // Then
        assert!(error.contains(expected), "{name}: {error}");
    }
    Ok(())
}

#[test]
fn scan_selected_chats_rejects_unsupported_reference_timezone() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-timezone.sqlite")?;
    let state = fake_state(&db_path, native_batch()?);
    let mut request = scan_request(
        &[chat("design-partners", 3, &["p1", "p2", "p3"])],
        &[],
        true,
        1,
        0,
    )?;
    request.reference_timezone = "America/Los_Angeles".to_owned();

    // When
    let error = state
        .scan_selected_chats_at(request, &db_path, &db_path)
        .err()
        .ok_or_else(|| "scan unexpectedly succeeded".to_owned())?
        .to_string();

    // Then
    assert!(error.contains("unsupported reference timezone"), "{error}");
    assert!(!error.contains("America/Los_Angeles"), "{error}");
    Ok(())
}

#[test]
fn scan_selected_chats_rejects_invalid_local_diagnostics_retention() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let store_path = dir.path().join("store-should-not-open.sqlite");
    let messages_db_path = dir.path().join("messages-should-not-open.sqlite");
    let request = serde_json::from_value(json!({
        "selectedChatIds": ["messages-chat-8b96e568c027a42d3b5c9e6e7710201f"],
        "selectedChats": [selected_chat_json(
            "messages-chat-8b96e568c027a42d3b5c9e6e7710201f",
            1,
            &["messages-participant-6044b729eea9fa126e78d421e4a41ac8"],
        )],
        "referenceTimezone": "Asia/Seoul",
        "referenceUnixSeconds": 1_782_352_400,
        "backfillPromptChatIds": [],
        "sourceExcerptsEnabled": true,
        "feedbackTextSnapshotsEnabled": false,
        "localDiagnosticsEnabled": true,
        "localDiagnosticsRetentionDays": 0,
        "capPolicy": {
            "mode": "refillForPending",
            "maxVisible": 1,
            "pendingCount": 0,
        },
    }))
    .map_err(|error| error.to_string())?;
    let runner = CountingCodexRunner::default();
    let adapter = RecordingProposalAdapter::default();

    // When
    let error = NativeBridgeState::default()
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
                    diagnostic: "test auth readiness".to_owned(),
                },
                codex_runner: &runner,
                proposal_adapter: &adapter,
            },
        )
        .err()
        .ok_or_else(|| "scan unexpectedly succeeded".to_owned())?
        .to_string();

    // Then
    if !error.contains("local diagnostics retention") {
        return Err(format!(
            "expected local diagnostics retention error, got: {error}"
        ));
    }
    if runner.calls() != 0 {
        return Err(format!(
            "provider ran before retention rejection: {}",
            runner.calls()
        ));
    }
    if adapter.created_count() != 0 {
        return Err(format!(
            "EventKit adapter ran before retention rejection: {}",
            adapter.created_count()
        ));
    }
    if store_path.exists() {
        return Err("invalid retention should be rejected before storage opens".to_owned());
    }
    if messages_db_path.exists() {
        return Err("invalid retention should be rejected before Messages opens".to_owned());
    }
    Ok(())
}
