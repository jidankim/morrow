use serde::{Deserialize, Serialize};

const LOGIN_STATUS_COMMAND: &str = "codex login status";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CodexAuthStatus {
    LoggedInUsingChatGpt,
    MissingCli,
    NotLoggedIn,
    Timeout,
    UnknownFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodexProviderAuthReadiness {
    pub status: CodexAuthStatus,
    pub ready: bool,
    pub command_surface: String,
    pub command_output_redacted: bool,
    pub diagnostic: String,
}

pub(super) fn readiness(status: CodexAuthStatus, ready: bool) -> CodexProviderAuthReadiness {
    readiness_with_redaction(status, ready, false)
}

pub(super) fn readiness_with_redaction(
    status: CodexAuthStatus,
    ready: bool,
    command_output_redacted: bool,
) -> CodexProviderAuthReadiness {
    CodexProviderAuthReadiness {
        status,
        ready,
        command_surface: LOGIN_STATUS_COMMAND.to_owned(),
        command_output_redacted,
        diagnostic: diagnostic(status),
    }
}

fn diagnostic(status: CodexAuthStatus) -> String {
    match status {
        CodexAuthStatus::LoggedInUsingChatGpt => "Codex CLI ChatGPT session is ready.".to_owned(),
        CodexAuthStatus::MissingCli => "Codex CLI was not found on PATH.".to_owned(),
        CodexAuthStatus::NotLoggedIn => "Codex CLI is installed but not logged in.".to_owned(),
        CodexAuthStatus::Timeout => "Codex CLI auth check timed out.".to_owned(),
        CodexAuthStatus::UnknownFailure => "Codex CLI auth check failed.".to_owned(),
    }
}
