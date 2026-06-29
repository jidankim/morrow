use super::{
    process::CodexAuthCommandOutput,
    readiness::{readiness_with_redaction, CodexAuthStatus, CodexProviderAuthReadiness},
};

pub(super) fn classify_completed_output(
    output: &CodexAuthCommandOutput,
) -> CodexProviderAuthReadiness {
    let combined = output.combined_output();
    if output.is_success() && mentions_chatgpt_login(&combined) {
        return readiness_with_redaction(
            CodexAuthStatus::LoggedInUsingChatGpt,
            true,
            output.has_output(),
        );
    }
    if mentions_missing_login(&combined) {
        return readiness_with_redaction(CodexAuthStatus::NotLoggedIn, false, output.has_output());
    }
    readiness_with_redaction(CodexAuthStatus::UnknownFailure, false, output.has_output())
}

fn mentions_chatgpt_login(output: &str) -> bool {
    output
        .to_ascii_lowercase()
        .contains("logged in using chatgpt")
}

fn mentions_missing_login(output: &str) -> bool {
    let normalized = output.to_ascii_lowercase();
    [
        "not logged in",
        "not authenticated",
        "login required",
        "run codex login",
        "please login",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}
