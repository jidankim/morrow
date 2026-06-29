use std::{path::PathBuf, time::Duration};

mod classification;
mod discovery;
mod pipes;
mod process;
mod readiness;

use classification::classify_completed_output;
use discovery::{resolve_executable, search_paths_from_env};
use readiness::readiness;

pub use process::{
    CodexAuthCommandOutput, CodexAuthCommandRunner, CodexLoginStatusRun,
    ProcessCodexAuthCommandRunner,
};
pub use readiness::{CodexAuthStatus, CodexProviderAuthReadiness};

const CODEX_EXECUTABLE: &str = "codex";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexAuthProbeOptions {
    executable_name: String,
    search_paths: Vec<PathBuf>,
    timeout: Duration,
}

impl CodexAuthProbeOptions {
    pub fn from_search_paths(search_paths: Vec<PathBuf>) -> Self {
        Self {
            executable_name: CODEX_EXECUTABLE.to_owned(),
            search_paths,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    pub fn with_timeout(self, timeout: Duration) -> Self {
        Self { timeout, ..self }
    }
}

impl Default for CodexAuthProbeOptions {
    fn default() -> Self {
        Self::from_search_paths(search_paths_from_env())
    }
}

pub fn probe_codex_provider_auth() -> CodexProviderAuthReadiness {
    let runner = ProcessCodexAuthCommandRunner;
    probe_codex_provider_auth_with_runner(&CodexAuthProbeOptions::default(), &runner)
}

pub fn probe_codex_provider_auth_with_runner(
    options: &CodexAuthProbeOptions,
    runner: &impl CodexAuthCommandRunner,
) -> CodexProviderAuthReadiness {
    let Some(executable) = resolve_executable(&options.executable_name, &options.search_paths)
    else {
        return readiness(CodexAuthStatus::MissingCli, false);
    };
    match runner.run_login_status(&executable, options.timeout) {
        CodexLoginStatusRun::Completed(output) => classify_completed_output(&output),
        CodexLoginStatusRun::MissingCli => readiness(CodexAuthStatus::MissingCli, false),
        CodexLoginStatusRun::TimedOut => readiness(CodexAuthStatus::Timeout, false),
        CodexLoginStatusRun::FailedToStart => readiness(CodexAuthStatus::UnknownFailure, false),
    }
}
