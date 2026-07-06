use std::{sync::OnceLock, time::Duration};

use super::{
    codex_auth, CodexAuthProbeOptions, CodexLoginLaunchReceipt, CodexProviderAuthReadiness,
    CodexSetupActionGuard, CodexSetupActionReceipt, ProcessCodexAuthCommandRunner,
};

const CODEX_INSTALL_TIMEOUT: Duration = Duration::from_secs(30);
const CODEX_LOGIN_LAUNCH_TIMEOUT: Duration = Duration::from_secs(300);

static CODEX_SETUP_GUARD: OnceLock<CodexSetupActionGuard> = OnceLock::new();

fn codex_setup_guard() -> &'static CodexSetupActionGuard {
    CODEX_SETUP_GUARD.get_or_init(CodexSetupActionGuard::default)
}

#[tauri::command]
pub fn check_provider_auth() -> CodexProviderAuthReadiness {
    codex_auth::probe_codex_provider_auth()
}

#[tauri::command]
pub fn install_codex_cli() -> CodexSetupActionReceipt {
    let runner = ProcessCodexAuthCommandRunner;
    codex_auth::install_codex_cli_with_runner(
        &CodexAuthProbeOptions::default(),
        &runner,
        codex_setup_guard(),
        CODEX_INSTALL_TIMEOUT,
    )
}

#[tauri::command]
pub fn start_codex_login() -> CodexLoginLaunchReceipt {
    let runner = ProcessCodexAuthCommandRunner;
    codex_auth::start_codex_login_with_runner(
        &CodexAuthProbeOptions::default(),
        &runner,
        codex_setup_guard(),
        CODEX_LOGIN_LAUNCH_TIMEOUT,
    )
}
