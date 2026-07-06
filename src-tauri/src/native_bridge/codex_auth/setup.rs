use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use serde::{Deserialize, Serialize};

use super::{
    discovery::resolve_executable,
    process::{CodexAuthCommandOutput, CodexAuthCommandRunner},
    CodexAuthProbeOptions,
};

const INSTALL_COMMAND_SURFACE: &str = "codex standalone installer";
const LOGIN_COMMAND_SURFACE: &str = "codex login";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodexSetupActionRun {
    Installed(CodexAuthCommandOutput),
    AlreadyInstalled,
    MissingCurl,
    CurlFailed(CodexAuthCommandOutput),
    InstallerFailed(CodexAuthCommandOutput),
    TimedOut(CodexAuthCommandOutput),
    Unsupported,
    Failed(CodexAuthCommandOutput),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodexLoginLaunchRun {
    Launched,
    MissingCli,
    FailedToStart(CodexAuthCommandOutput),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CodexSetupActionStatus {
    Installed,
    AlreadyInstalled,
    AlreadyRunning,
    Failed,
    Timeout,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodexSetupActionReceipt {
    pub status: CodexSetupActionStatus,
    pub command_surface: String,
    pub command_output_redacted: bool,
    pub diagnostic: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CodexLoginLaunchStatus {
    Launched,
    AlreadyRunning,
    MissingCli,
    FailedToStart,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodexLoginLaunchReceipt {
    pub status: CodexLoginLaunchStatus,
    pub command_surface: String,
    pub command_output_redacted: bool,
    pub diagnostic: String,
}

#[derive(Debug, Default)]
pub struct CodexSetupActionGuard {
    action_running: Arc<AtomicBool>,
}

impl CodexSetupActionGuard {
    fn begin_install(&self) -> Option<RunningAction> {
        RunningAction::begin(Arc::clone(&self.action_running))
    }

    fn begin_login(&self) -> Option<CodexLoginRunningGuard> {
        CodexLoginRunningGuard::begin(Arc::clone(&self.action_running))
    }
}

struct RunningAction {
    running: Arc<AtomicBool>,
}

impl RunningAction {
    fn begin(running: Arc<AtomicBool>) -> Option<Self> {
        running
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
            .then_some(Self { running })
    }
}

impl Drop for RunningAction {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
    }
}

#[derive(Debug)]
pub struct CodexLoginRunningGuard {
    running: Arc<AtomicBool>,
}

impl CodexLoginRunningGuard {
    fn begin(running: Arc<AtomicBool>) -> Option<Self> {
        running
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
            .then_some(Self { running })
    }
}

impl Drop for CodexLoginRunningGuard {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
    }
}

pub fn install_codex_cli_with_runner(
    options: &CodexAuthProbeOptions,
    runner: &impl CodexAuthCommandRunner,
    guard: &CodexSetupActionGuard,
    timeout: Duration,
) -> CodexSetupActionReceipt {
    if resolve_executable(&options.executable_name, &options.search_paths).is_some() {
        return setup_receipt(
            CodexSetupActionStatus::AlreadyInstalled,
            "Codex CLI is already installed.",
        );
    }
    let Some(_running) = guard.begin_install() else {
        return setup_receipt(
            CodexSetupActionStatus::AlreadyRunning,
            "Codex CLI setup is already running.",
        );
    };
    match runner.run_codex_install(timeout) {
        CodexSetupActionRun::Installed(_output) => {
            if resolve_executable(&options.executable_name, &options.search_paths).is_some() {
                setup_receipt(
                    CodexSetupActionStatus::Installed,
                    "Codex CLI installed successfully.",
                )
            } else {
                setup_receipt(
                    CodexSetupActionStatus::Failed,
                    "Codex CLI setup completed but the CLI was not found.",
                )
            }
        }
        CodexSetupActionRun::AlreadyInstalled => setup_receipt(
            CodexSetupActionStatus::AlreadyInstalled,
            "Codex CLI is already installed.",
        ),
        CodexSetupActionRun::MissingCurl => setup_receipt(
            CodexSetupActionStatus::Failed,
            "Codex CLI installer could not start.",
        ),
        CodexSetupActionRun::CurlFailed(_output) => setup_receipt(
            CodexSetupActionStatus::Failed,
            "Codex CLI installer download failed.",
        ),
        CodexSetupActionRun::InstallerFailed(_output) => setup_receipt(
            CodexSetupActionStatus::Failed,
            "Codex CLI installer failed.",
        ),
        CodexSetupActionRun::TimedOut(_output) => setup_receipt(
            CodexSetupActionStatus::Timeout,
            "Codex CLI installer timed out and was stopped.",
        ),
        CodexSetupActionRun::Unsupported => setup_receipt(
            CodexSetupActionStatus::Unsupported,
            "Codex CLI setup is unsupported on this platform.",
        ),
        CodexSetupActionRun::Failed(_output) => {
            setup_receipt(CodexSetupActionStatus::Failed, "Codex CLI setup failed.")
        }
    }
}

pub fn start_codex_login_with_runner(
    options: &CodexAuthProbeOptions,
    runner: &impl CodexAuthCommandRunner,
    guard: &CodexSetupActionGuard,
    timeout: Duration,
) -> CodexLoginLaunchReceipt {
    let Some(executable) = resolve_executable(&options.executable_name, &options.search_paths)
    else {
        return login_receipt(
            CodexLoginLaunchStatus::MissingCli,
            "Codex CLI was not found on PATH.",
        );
    };
    let Some(running) = guard.begin_login() else {
        return login_receipt(
            CodexLoginLaunchStatus::AlreadyRunning,
            "Codex login is already running.",
        );
    };
    match runner.launch_codex_login_with_guard(&executable, timeout, running) {
        CodexLoginLaunchRun::Launched => login_receipt(
            CodexLoginLaunchStatus::Launched,
            "Codex login started successfully.",
        ),
        CodexLoginLaunchRun::MissingCli => login_receipt(
            CodexLoginLaunchStatus::MissingCli,
            "Codex CLI was not found on PATH.",
        ),
        CodexLoginLaunchRun::FailedToStart(_output) => login_receipt(
            CodexLoginLaunchStatus::FailedToStart,
            "Codex login could not be started.",
        ),
    }
}

fn setup_receipt(status: CodexSetupActionStatus, diagnostic: &str) -> CodexSetupActionReceipt {
    CodexSetupActionReceipt {
        status,
        command_surface: INSTALL_COMMAND_SURFACE.to_owned(),
        command_output_redacted: true,
        diagnostic: diagnostic.to_owned(),
    }
}

fn login_receipt(status: CodexLoginLaunchStatus, diagnostic: &str) -> CodexLoginLaunchReceipt {
    CodexLoginLaunchReceipt {
        status,
        command_surface: LOGIN_COMMAND_SURFACE.to_owned(),
        command_output_redacted: true,
        diagnostic: diagnostic.to_owned(),
    }
}
