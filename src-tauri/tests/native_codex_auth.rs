use std::{
    cell::RefCell,
    ffi::OsString,
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, MutexGuard},
    thread,
    time::{Duration, Instant},
};

use morrow_lib::native_bridge::{
    install_codex_cli_with_runner, probe_codex_provider_auth_with_runner,
    start_codex_login_with_runner, CodexAuthCommandOutput, CodexAuthCommandRunner,
    CodexAuthProbeOptions, CodexAuthStatus, CodexLoginLaunchRun, CodexLoginLaunchStatus,
    CodexLoginStatusRun, CodexSetupActionGuard, CodexSetupActionRun, CodexSetupActionStatus,
    ProcessCodexAuthCommandRunner,
};
use serde::Serialize;

static ENV_LOCK: Mutex<()> = Mutex::new(());
const FORBIDDEN_CHILD_ENV_VARS: [&str; 6] = [
    "CODEX_ACCESS_TOKEN",
    "OPENAI_API_KEY",
    "OPENAI_AUTH_TOKEN",
    "ANTHROPIC_API_KEY",
    "AUTHORIZATION",
    "COOKIE",
];

#[derive(Debug)]
struct FakeCodexAuthRunner {
    result: CodexLoginStatusRun,
    install_result: CodexSetupActionRun,
    install_codex_path: Option<PathBuf>,
    login_result: CodexLoginLaunchRun,
    calls: RefCell<Vec<(PathBuf, Duration)>>,
    install_calls: RefCell<Vec<Duration>>,
    login_calls: RefCell<Vec<(PathBuf, Duration)>>,
}

impl FakeCodexAuthRunner {
    fn new(result: CodexLoginStatusRun) -> Self {
        Self {
            result,
            install_result: CodexSetupActionRun::Unsupported,
            install_codex_path: None,
            login_result: CodexLoginLaunchRun::FailedToStart(CodexAuthCommandOutput::new(
                None, "", "",
            )),
            calls: RefCell::new(Vec::new()),
            install_calls: RefCell::new(Vec::new()),
            login_calls: RefCell::new(Vec::new()),
        }
    }

    fn with_install_result(mut self, install_result: CodexSetupActionRun) -> Self {
        self.install_result = install_result;
        self
    }

    fn with_installed_codex_path(mut self, codex_path: PathBuf) -> Self {
        self.install_codex_path = Some(codex_path);
        self
    }

    fn with_login_result(mut self, login_result: CodexLoginLaunchRun) -> Self {
        self.login_result = login_result;
        self
    }

    fn calls(&self) -> Vec<(PathBuf, Duration)> {
        self.calls.borrow().clone()
    }

    fn install_calls(&self) -> Vec<Duration> {
        self.install_calls.borrow().clone()
    }

    fn login_calls(&self) -> Vec<(PathBuf, Duration)> {
        self.login_calls.borrow().clone()
    }
}

impl CodexAuthCommandRunner for FakeCodexAuthRunner {
    fn run_login_status(&self, executable: &Path, timeout: Duration) -> CodexLoginStatusRun {
        self.calls
            .borrow_mut()
            .push((executable.to_path_buf(), timeout));
        self.result.clone()
    }

    fn run_codex_install(&self, timeout: Duration) -> CodexSetupActionRun {
        self.install_calls.borrow_mut().push(timeout);
        if matches!(self.install_result, CodexSetupActionRun::Installed(_)) {
            if let Some(codex_path) = &self.install_codex_path {
                drop(write_executable(codex_path, "#!/bin/sh\nexit 0\n"));
            }
        }
        self.install_result.clone()
    }

    fn launch_codex_login(&self, executable: &Path, timeout: Duration) -> CodexLoginLaunchRun {
        self.login_calls
            .borrow_mut()
            .push((executable.to_path_buf(), timeout));
        self.login_result.clone()
    }
}

#[test]
fn codex_setup_install_reports_success_after_reprobe() -> Result<(), String> {
    // Given
    let temp_dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let codex_path = temp_dir.path().join("codex");
    let options = CodexAuthProbeOptions::from_search_paths(vec![temp_dir.path().to_path_buf()]);
    let guard = CodexSetupActionGuard::default();
    let runner = FakeCodexAuthRunner::new(CodexLoginStatusRun::MissingCli)
        .with_install_result(CodexSetupActionRun::Installed(CodexAuthCommandOutput::new(
            Some(0),
            "installed codex_access_token=leaked",
            "",
        )))
        .with_installed_codex_path(codex_path);

    // When
    let receipt = install_codex_cli_with_runner(&options, &runner, &guard, Duration::from_secs(30));

    // Then
    assert_eq!(receipt.status, CodexSetupActionStatus::Installed);
    assert_eq!(receipt.command_surface, "codex standalone installer");
    assert!(receipt.command_output_redacted);
    assert_eq!(runner.install_calls(), vec![Duration::from_secs(30)]);
    Ok(())
}

#[test]
fn codex_setup_install_reports_failed_when_curl_missing() {
    // Given
    let options = CodexAuthProbeOptions::from_search_paths(Vec::new());
    let guard = CodexSetupActionGuard::default();
    let runner = FakeCodexAuthRunner::new(CodexLoginStatusRun::MissingCli)
        .with_install_result(CodexSetupActionRun::MissingCurl);

    // When
    let receipt = install_codex_cli_with_runner(&options, &runner, &guard, Duration::from_secs(30));

    // Then
    assert_eq!(receipt.status, CodexSetupActionStatus::Failed);
    assert_eq!(receipt.diagnostic, "Codex CLI installer could not start.");
    assert!(receipt.command_output_redacted);
}

#[test]
fn codex_setup_install_reports_failed_when_curl_fails() {
    // Given
    let options = CodexAuthProbeOptions::from_search_paths(Vec::new());
    let guard = CodexSetupActionGuard::default();
    let runner = FakeCodexAuthRunner::new(CodexLoginStatusRun::MissingCli).with_install_result(
        CodexSetupActionRun::CurlFailed(CodexAuthCommandOutput::new(
            Some(22),
            "",
            "curl failed CODEX_ACCESS_TOKEN=leaked",
        )),
    );

    // When
    let receipt = install_codex_cli_with_runner(&options, &runner, &guard, Duration::from_secs(30));

    // Then
    assert_eq!(receipt.status, CodexSetupActionStatus::Failed);
    assert_eq!(receipt.diagnostic, "Codex CLI installer download failed.");
    assert!(receipt.command_output_redacted);
}

#[test]
fn codex_setup_install_reports_failed_when_installer_fails() {
    // Given
    let options = CodexAuthProbeOptions::from_search_paths(Vec::new());
    let guard = CodexSetupActionGuard::default();
    let runner = FakeCodexAuthRunner::new(CodexLoginStatusRun::MissingCli).with_install_result(
        CodexSetupActionRun::InstallerFailed(CodexAuthCommandOutput::new(
            Some(1),
            "installer mentioned ~/.codex/auth.json",
            "",
        )),
    );

    // When
    let receipt = install_codex_cli_with_runner(&options, &runner, &guard, Duration::from_secs(30));

    // Then
    assert_eq!(receipt.status, CodexSetupActionStatus::Failed);
    assert_eq!(receipt.diagnostic, "Codex CLI installer failed.");
    assert!(receipt.command_output_redacted);
}

#[test]
fn codex_setup_install_reports_timeout_and_redacted_cleanup() {
    // Given
    let temp_dir = tempfile::tempdir().expect("temp dir exists");
    let codex_path = temp_dir.path().join("codex");
    let options = CodexAuthProbeOptions::from_search_paths(vec![temp_dir.path().to_path_buf()]);
    let guard = CodexSetupActionGuard::default();
    let runner = FakeCodexAuthRunner::new(CodexLoginStatusRun::MissingCli).with_install_result(
        CodexSetupActionRun::TimedOut(CodexAuthCommandOutput::new(None, "timeout sk-leaked", "")),
    );
    let retry_runner = FakeCodexAuthRunner::new(CodexLoginStatusRun::MissingCli)
        .with_install_result(CodexSetupActionRun::Installed(CodexAuthCommandOutput::new(
            Some(0),
            "",
            "",
        )))
        .with_installed_codex_path(codex_path);

    // When
    let receipt = install_codex_cli_with_runner(&options, &runner, &guard, Duration::from_secs(1));
    let retry_receipt =
        install_codex_cli_with_runner(&options, &retry_runner, &guard, Duration::from_secs(30));

    // Then
    assert_eq!(receipt.status, CodexSetupActionStatus::Timeout);
    assert_eq!(
        receipt.diagnostic,
        "Codex CLI installer timed out and was stopped."
    );
    assert!(receipt.command_output_redacted);
    assert_eq!(retry_receipt.status, CodexSetupActionStatus::Installed);
    assert_eq!(runner.install_calls(), vec![Duration::from_secs(1)]);
    assert_eq!(retry_runner.install_calls(), vec![Duration::from_secs(30)]);
}

#[test]
fn codex_setup_install_reports_already_installed_without_running_installer() -> Result<(), String> {
    // Given
    let fixture = CodexAuthFixture::new()?;
    let guard = CodexSetupActionGuard::default();
    let runner = FakeCodexAuthRunner::new(CodexLoginStatusRun::MissingCli).with_install_result(
        CodexSetupActionRun::Installed(CodexAuthCommandOutput::new(Some(0), "", "")),
    );

    // When
    let receipt =
        install_codex_cli_with_runner(&fixture.options, &runner, &guard, Duration::from_secs(30));

    // Then
    assert_eq!(receipt.status, CodexSetupActionStatus::AlreadyInstalled);
    assert!(runner.install_calls().is_empty());
    Ok(())
}

#[test]
fn codex_setup_install_reports_already_running_for_duplicate_install() {
    // Given
    let temp_dir = tempfile::tempdir().expect("temp dir exists");
    let codex_path = temp_dir.path().join("codex");
    let options = CodexAuthProbeOptions::from_search_paths(vec![temp_dir.path().to_path_buf()]);
    let guard = CodexSetupActionGuard::default();
    let runner = ReentrantInstallRunner {
        options: &options,
        guard: &guard,
        codex_path,
        nested_receipt: RefCell::new(None),
    };

    // When
    let receipt = install_codex_cli_with_runner(&options, &runner, &guard, Duration::from_secs(30));

    // Then
    let nested = runner
        .nested_receipt
        .borrow()
        .clone()
        .ok_or("missing nested duplicate receipt")
        .expect("duplicate install receipt exists");
    assert_eq!(nested.status, CodexSetupActionStatus::AlreadyRunning);
    assert_eq!(receipt.status, CodexSetupActionStatus::Installed);
}

#[test]
fn codex_setup_login_reports_launched() -> Result<(), String> {
    // Given
    let fixture = CodexAuthFixture::new()?;
    let guard = CodexSetupActionGuard::default();
    let runner = FakeCodexAuthRunner::new(CodexLoginStatusRun::MissingCli)
        .with_login_result(CodexLoginLaunchRun::Launched);

    // When
    let receipt =
        start_codex_login_with_runner(&fixture.options, &runner, &guard, Duration::from_secs(300));

    // Then
    assert_eq!(receipt.status, CodexLoginLaunchStatus::Launched);
    assert_eq!(receipt.command_surface, "codex login");
    assert_eq!(
        runner.login_calls(),
        vec![(fixture.codex_path, Duration::from_secs(300))]
    );
    Ok(())
}

#[test]
fn codex_setup_login_reports_already_running_for_duplicate_login() -> Result<(), String> {
    // Given
    let fixture = CodexAuthFixture::new()?;
    let guard = CodexSetupActionGuard::default();
    let runner = ReentrantLoginRunner {
        options: &fixture.options,
        guard: &guard,
        nested_receipt: RefCell::new(None),
    };

    // When
    let receipt =
        start_codex_login_with_runner(&fixture.options, &runner, &guard, Duration::from_secs(300));

    // Then
    let nested = runner
        .nested_receipt
        .borrow()
        .clone()
        .ok_or("missing nested duplicate receipt")
        .expect("duplicate login receipt exists");
    assert_eq!(nested.status, CodexLoginLaunchStatus::AlreadyRunning);
    assert_eq!(receipt.status, CodexLoginLaunchStatus::Launched);
    Ok(())
}

#[test]
fn codex_setup_install_blocks_reentrant_login() -> Result<(), String> {
    // Given
    let temp_dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let codex_path = temp_dir.path().join("codex");
    let options = CodexAuthProbeOptions::from_search_paths(vec![temp_dir.path().to_path_buf()]);
    let guard = CodexSetupActionGuard::default();
    let runner = ReentrantInstallThenLoginRunner {
        options: &options,
        guard: &guard,
        codex_path,
        nested_receipt: RefCell::new(None),
    };

    // When
    let receipt = install_codex_cli_with_runner(&options, &runner, &guard, Duration::from_secs(30));

    // Then
    let nested = runner
        .nested_receipt
        .borrow()
        .clone()
        .ok_or("missing nested login receipt")?;
    assert_eq!(nested.status, CodexLoginLaunchStatus::AlreadyRunning);
    assert_eq!(receipt.status, CodexSetupActionStatus::Installed);
    Ok(())
}

#[test]
fn codex_setup_login_reports_missing_cli_without_spawn() {
    // Given
    let options = CodexAuthProbeOptions::from_search_paths(Vec::new());
    let guard = CodexSetupActionGuard::default();
    let runner = FakeCodexAuthRunner::new(CodexLoginStatusRun::MissingCli)
        .with_login_result(CodexLoginLaunchRun::Launched);

    // When
    let receipt =
        start_codex_login_with_runner(&options, &runner, &guard, Duration::from_secs(300));

    // Then
    assert_eq!(receipt.status, CodexLoginLaunchStatus::MissingCli);
    assert!(runner.login_calls().is_empty());
}

#[test]
fn codex_setup_login_reports_failed_spawn() -> Result<(), String> {
    // Given
    let fixture = CodexAuthFixture::new()?;
    let guard = CodexSetupActionGuard::default();
    let runner = FakeCodexAuthRunner::new(CodexLoginStatusRun::MissingCli).with_login_result(
        CodexLoginLaunchRun::FailedToStart(CodexAuthCommandOutput::new(
            None,
            "open https://auth.openai.com/device and enter ABCD-EFGH",
            "",
        )),
    );

    // When
    let receipt =
        start_codex_login_with_runner(&fixture.options, &runner, &guard, Duration::from_secs(300));

    // Then
    assert_eq!(receipt.status, CodexLoginLaunchStatus::FailedToStart);
    assert_eq!(receipt.diagnostic, "Codex login could not be started.");
    assert!(receipt.command_output_redacted);
    Ok(())
}

#[test]
fn codex_setup_receipts_redact_installer_and_login_output() -> Result<(), String> {
    // Given
    let fixture = CodexAuthFixture::new()?;
    let install_options = CodexAuthProbeOptions::from_search_paths(Vec::new());
    let install_guard = CodexSetupActionGuard::default();
    let login_guard = CodexSetupActionGuard::default();
    let runner = FakeCodexAuthRunner::new(CodexLoginStatusRun::MissingCli)
        .with_install_result(CodexSetupActionRun::InstallerFailed(
            CodexAuthCommandOutput::new(
                Some(1),
                "codex_access_token=leaked CODEX_ACCESS_TOKEN auth.json ~/.codex sk-leaked",
                "",
            ),
        ))
        .with_login_result(CodexLoginLaunchRun::FailedToStart(
            CodexAuthCommandOutput::new(
                None,
                "https://auth.openai.com/device raw login code ABCD-EFGH",
                "",
            ),
        ));

    // When
    let install_receipt = install_codex_cli_with_runner(
        &install_options,
        &runner,
        &install_guard,
        Duration::from_secs(30),
    );
    let login_receipt = start_codex_login_with_runner(
        &fixture.options,
        &runner,
        &login_guard,
        Duration::from_secs(300),
    );

    // Then
    assert_receipt_is_sanitized(&install_receipt)?;
    assert_receipt_is_sanitized(&login_receipt)?;
    Ok(())
}

struct ReentrantInstallRunner<'a> {
    options: &'a CodexAuthProbeOptions,
    guard: &'a CodexSetupActionGuard,
    codex_path: PathBuf,
    nested_receipt: RefCell<Option<morrow_lib::native_bridge::CodexSetupActionReceipt>>,
}

impl CodexAuthCommandRunner for ReentrantInstallRunner<'_> {
    fn run_login_status(&self, _executable: &Path, _timeout: Duration) -> CodexLoginStatusRun {
        CodexLoginStatusRun::MissingCli
    }

    fn run_codex_install(&self, timeout: Duration) -> CodexSetupActionRun {
        let nested = install_codex_cli_with_runner(self.options, self, self.guard, timeout);
        self.nested_receipt.borrow_mut().replace(nested);
        drop(write_executable(&self.codex_path, "#!/bin/sh\nexit 0\n"));
        CodexSetupActionRun::Installed(CodexAuthCommandOutput::new(Some(0), "", ""))
    }

    fn launch_codex_login(&self, _executable: &Path, _timeout: Duration) -> CodexLoginLaunchRun {
        CodexLoginLaunchRun::FailedToStart(CodexAuthCommandOutput::new(None, "", ""))
    }
}

struct ReentrantLoginRunner<'a> {
    options: &'a CodexAuthProbeOptions,
    guard: &'a CodexSetupActionGuard,
    nested_receipt: RefCell<Option<morrow_lib::native_bridge::CodexLoginLaunchReceipt>>,
}

impl CodexAuthCommandRunner for ReentrantLoginRunner<'_> {
    fn run_login_status(&self, _executable: &Path, _timeout: Duration) -> CodexLoginStatusRun {
        CodexLoginStatusRun::MissingCli
    }

    fn run_codex_install(&self, _timeout: Duration) -> CodexSetupActionRun {
        CodexSetupActionRun::Unsupported
    }

    fn launch_codex_login(&self, executable: &Path, timeout: Duration) -> CodexLoginLaunchRun {
        let nested = start_codex_login_with_runner(self.options, self, self.guard, timeout);
        self.nested_receipt.borrow_mut().replace(nested);
        assert_eq!(
            executable.file_name().and_then(|name| name.to_str()),
            Some("codex")
        );
        CodexLoginLaunchRun::Launched
    }
}

struct ReentrantInstallThenLoginRunner<'a> {
    options: &'a CodexAuthProbeOptions,
    guard: &'a CodexSetupActionGuard,
    codex_path: PathBuf,
    nested_receipt: RefCell<Option<morrow_lib::native_bridge::CodexLoginLaunchReceipt>>,
}

impl CodexAuthCommandRunner for ReentrantInstallThenLoginRunner<'_> {
    fn run_login_status(&self, _executable: &Path, _timeout: Duration) -> CodexLoginStatusRun {
        CodexLoginStatusRun::MissingCli
    }

    fn run_codex_install(&self, timeout: Duration) -> CodexSetupActionRun {
        drop(write_executable(&self.codex_path, "#!/bin/sh\nexit 0\n"));
        let nested = start_codex_login_with_runner(self.options, self, self.guard, timeout);
        self.nested_receipt.borrow_mut().replace(nested);
        CodexSetupActionRun::Installed(CodexAuthCommandOutput::new(Some(0), "", ""))
    }

    fn launch_codex_login(&self, _executable: &Path, _timeout: Duration) -> CodexLoginLaunchRun {
        CodexLoginLaunchRun::Launched
    }
}

fn assert_receipt_is_sanitized<T>(receipt: &T) -> Result<(), String>
where
    T: Serialize + Debug,
{
    let serialized = serde_json::to_string(receipt).map_err(|error| error.to_string())?;
    let debug = format!("{receipt:?}");
    for forbidden in [
        "codex_access_token=leaked",
        "CODEX_ACCESS_TOKEN",
        "auth.json",
        "~/.codex",
        "sk-",
        "https://auth.openai.com/device",
        "raw login code",
        "ABCD-EFGH",
        "\"stdout\"",
        "\"stderr\"",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "serialized receipt leaked {forbidden}: {serialized}"
        );
        assert!(
            !debug.contains(forbidden),
            "debug receipt leaked {forbidden}: {debug}"
        );
    }
    Ok(())
}

#[test]
fn codex_auth_probe_reports_chatgpt_login() -> Result<(), String> {
    // Given
    let fixture = CodexAuthFixture::new()?;
    let runner = FakeCodexAuthRunner::new(CodexLoginStatusRun::Completed(
        CodexAuthCommandOutput::new(Some(0), "Logged in using ChatGPT\n", ""),
    ));

    // When
    let readiness = probe_codex_provider_auth_with_runner(&fixture.options, &runner);

    // Then
    assert_eq!(readiness.status, CodexAuthStatus::LoggedInUsingChatGpt);
    assert!(readiness.ready);
    let calls = runner.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, fixture.codex_path);
    Ok(())
}

#[test]
fn codex_auth_probe_reports_missing_cli() {
    // Given
    let runner = FakeCodexAuthRunner::new(CodexLoginStatusRun::Completed(
        CodexAuthCommandOutput::new(Some(0), "Logged in using ChatGPT\n", ""),
    ));
    let options = CodexAuthProbeOptions::from_search_paths(Vec::new());

    // When
    let readiness = probe_codex_provider_auth_with_runner(&options, &runner);

    // Then
    assert_eq!(readiness.status, CodexAuthStatus::MissingCli);
    assert!(!readiness.ready);
    assert!(runner.calls().is_empty());
}

#[test]
fn codex_auth_probe_finds_user_installed_cli_when_packaged_app_path_omits_shell_bins(
) -> Result<(), String> {
    // Given
    let _env_lock = lock_env()?;
    let home_dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let user_bin_dir = home_dir.path().join(".local").join("bin");
    std::fs::create_dir_all(&user_bin_dir).map_err(|error| error.to_string())?;
    let codex_path = user_bin_dir.join("codex");
    std::fs::write(&codex_path, "#!/bin/sh\nexit 0\n").map_err(|error| error.to_string())?;
    mark_executable(&codex_path)?;
    let runner = FakeCodexAuthRunner::new(CodexLoginStatusRun::Completed(
        CodexAuthCommandOutput::new(Some(0), "Logged in using ChatGPT\n", ""),
    ));
    let options = {
        let _home_guard = EnvVarGuard::set("HOME", home_dir.path().as_os_str());
        let _path_guard = EnvVarGuard::set("PATH", "/usr/bin:/bin");
        CodexAuthProbeOptions::default()
    };

    // When
    let readiness = probe_codex_provider_auth_with_runner(&options, &runner);

    // Then
    assert_eq!(readiness.status, CodexAuthStatus::LoggedInUsingChatGpt);
    assert!(readiness.ready);
    let calls = runner.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, codex_path);
    Ok(())
}

#[test]
fn codex_auth_probe_reports_not_logged_in() -> Result<(), String> {
    // Given
    let fixture = CodexAuthFixture::new()?;
    let runner = FakeCodexAuthRunner::new(CodexLoginStatusRun::Completed(
        CodexAuthCommandOutput::new(Some(1), "Not logged in. Run codex login.\n", ""),
    ));

    // When
    let readiness = probe_codex_provider_auth_with_runner(&fixture.options, &runner);

    // Then
    assert_eq!(readiness.status, CodexAuthStatus::NotLoggedIn);
    assert!(!readiness.ready);
    Ok(())
}

#[test]
fn codex_auth_probe_redacts_output() -> Result<(), String> {
    // Given
    let fixture = CodexAuthFixture::new()?;
    let runner =
        FakeCodexAuthRunner::new(CodexLoginStatusRun::Completed(CodexAuthCommandOutput::new(
            Some(0),
            "Logged in using ChatGPT with codex-access-token-secret\n",
            "sk-secret-never-returned",
        )));

    // When
    let readiness = probe_codex_provider_auth_with_runner(&fixture.options, &runner);

    // Then
    assert_eq!(readiness.status, CodexAuthStatus::LoggedInUsingChatGpt);
    assert!(readiness.command_output_redacted);
    let serialized = serde_json::to_string(&readiness).map_err(|error| error.to_string())?;
    let debug = format!("{readiness:?}");
    for forbidden in [
        "codex-access-token-secret",
        "sk-secret-never-returned",
        "Logged in using ChatGPT",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "serialized readiness leaked {forbidden}: {serialized}"
        );
        assert!(
            !debug.contains(forbidden),
            "debug readiness leaked {forbidden}: {debug}"
        );
    }
    Ok(())
}

#[test]
fn codex_auth_command_output_debug_redacts_raw_output() {
    // Given
    let output = CodexAuthCommandOutput::new(
        Some(0),
        "Logged in using ChatGPT with codex-access-token-secret\n",
        "sk-secret-never-returned",
    );

    // When
    let debug = format!("{output:?}");

    // Then
    assert!(debug.contains("CodexAuthCommandOutput"));
    assert!(debug.contains("exit_code"));
    assert!(debug.contains("<redacted>"));
    for forbidden in [
        "codex-access-token-secret",
        "sk-secret-never-returned",
        "Logged in using ChatGPT",
    ] {
        assert!(
            !debug.contains(forbidden),
            "debug output leaked {forbidden}: {debug}"
        );
    }
}

#[test]
fn codex_auth_login_status_run_debug_redacts_completed_output() {
    // Given
    let run = CodexLoginStatusRun::Completed(CodexAuthCommandOutput::new(
        Some(0),
        "Logged in using ChatGPT with codex-access-token-secret\n",
        "sk-secret-never-returned",
    ));

    // When
    let debug = format!("{run:?}");

    // Then
    assert!(debug.contains("Completed"));
    assert!(debug.contains("<redacted>"));
    for forbidden in [
        "codex-access-token-secret",
        "sk-secret-never-returned",
        "Logged in using ChatGPT",
    ] {
        assert!(
            !debug.contains(forbidden),
            "debug output leaked {forbidden}: {debug}"
        );
    }
}

#[test]
fn codex_auth_process_runner_drains_noisy_status_output_without_timeout() -> Result<(), String> {
    // Given
    let fixture = CodexAuthFixture::with_script(
        r#"#!/bin/sh
i=0
while [ "$i" -lt 4096 ]; do
  printf 'Logged in using ChatGPT noisy stdout line %04d padding padding padding padding\n' "$i"
  printf 'noisy stderr line %04d padding padding padding padding\n' "$i" >&2
  i=$((i + 1))
done
exit 0
"#,
    )?;
    let options = fixture.options.clone().with_timeout(Duration::from_secs(5));
    let runner = ProcessCodexAuthCommandRunner;

    // When
    let readiness = probe_codex_provider_auth_with_runner(&options, &runner);

    // Then
    assert_eq!(readiness.status, CodexAuthStatus::LoggedInUsingChatGpt);
    assert!(readiness.ready);
    assert!(readiness.command_output_redacted);
    Ok(())
}

#[test]
fn codex_auth_process_runner_fails_closed_on_excessive_status_output() -> Result<(), String> {
    // Given
    let fixture = CodexAuthFixture::with_script(
        r#"#!/bin/sh
if [ "$1" = "login" ] && [ "$2" = "status" ]; then
  printf 'Logged in using ChatGPT\n'
  dd if=/dev/zero bs=1024 count=8192 2>/dev/null | tr '\000' A
  exit 0
fi
exit 1
"#,
    )?;
    let options = fixture.options.clone().with_timeout(Duration::from_secs(5));
    let runner = ProcessCodexAuthCommandRunner;

    // When
    let readiness = probe_codex_provider_auth_with_runner(&options, &runner);

    // Then
    assert_eq!(readiness.status, CodexAuthStatus::UnknownFailure);
    assert!(!readiness.ready);
    Ok(())
}

#[test]
fn codex_auth_process_runner_does_not_wait_for_pipe_holding_descendant() -> Result<(), String> {
    // Given
    let fixture = CodexAuthFixture::with_script(
        r#"#!/bin/sh
if [ "$1" = "login" ] && [ "$2" = "status" ]; then
  (sleep 3) &
  printf 'Logged in using ChatGPT\n'
  exit 0
fi
exit 1
"#,
    )?;
    let options = fixture.options.clone().with_timeout(Duration::from_secs(2));
    let runner = ProcessCodexAuthCommandRunner;

    // When
    let started = Instant::now();
    let readiness = probe_codex_provider_auth_with_runner(&options, &runner);
    let elapsed = started.elapsed();

    // Then
    assert!(
        elapsed < Duration::from_secs(1),
        "login status waited for a descendant-held pipe for {elapsed:?}"
    );
    assert_eq!(readiness.status, CodexAuthStatus::UnknownFailure);
    assert!(!readiness.ready);
    Ok(())
}

#[test]
fn codex_setup_process_runner_fails_closed_on_excessive_curl_output() -> Result<(), String> {
    // Given
    let _env_lock = lock_env()?;
    let temp_dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let fake_bin = temp_dir.path().join("bin");
    let home_dir = temp_dir.path().join("home");
    fs::create_dir_all(&fake_bin).map_err(|error| error.to_string())?;
    fs::create_dir_all(&home_dir).map_err(|error| error.to_string())?;
    let curl_path = fake_bin.join("curl");
    write_executable(
        &curl_path,
        r#"#!/bin/sh
cat <<'INSTALLER'
mkdir -p "$CODEX_INSTALL_DIR"
cat > "$CODEX_INSTALL_DIR/codex" <<'CODEX'
#!/bin/sh
printf 'Logged in using ChatGPT\n'
exit 0
CODEX
chmod 700 "$CODEX_INSTALL_DIR/codex"
exit 0
INSTALLER
dd if=/dev/zero bs=1024 count=8192 2>/dev/null | tr '\000' '#'
"#,
    )?;
    let test_path = format!("{}:/usr/bin:/bin", fake_bin.display());
    let _path_guard = EnvVarGuard::set("PATH", test_path);
    let _home_guard = EnvVarGuard::set("HOME", home_dir.as_os_str());
    let options = CodexAuthProbeOptions::default().with_timeout(Duration::from_secs(5));
    let guard = CodexSetupActionGuard::default();
    let runner = ProcessCodexAuthCommandRunner;

    // When
    let receipt = install_codex_cli_with_runner(&options, &runner, &guard, Duration::from_secs(5));

    // Then
    assert_eq!(receipt.status, CodexSetupActionStatus::Failed);
    assert!(!home_dir.join(".local").join("bin").join("codex").exists());
    assert!(receipt.command_output_redacted);
    Ok(())
}

#[test]
fn codex_setup_process_runner_sanitizes_child_environments() -> Result<(), String> {
    // Given
    let _env_lock = lock_env()?;
    let temp_dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let fake_bin = temp_dir.path().join("bin");
    let home_dir = temp_dir.path().join("home");
    let capture_dir = temp_dir.path().join("capture");
    fs::create_dir_all(&fake_bin).map_err(|error| error.to_string())?;
    fs::create_dir_all(&home_dir).map_err(|error| error.to_string())?;
    fs::create_dir_all(&capture_dir).map_err(|error| error.to_string())?;
    let curl_path = fake_bin.join("curl");
    write_executable(&curl_path, &sanitized_env_curl_script(&capture_dir))?;
    let test_path = format!("{}:/usr/bin:/bin", fake_bin.display());
    let _path_guard = EnvVarGuard::set("PATH", test_path);
    let _home_guard = EnvVarGuard::set("HOME", home_dir.as_os_str());
    let _codex_token_guard = EnvVarGuard::set("CODEX_ACCESS_TOKEN", "secret-codex-token");
    let _openai_key_guard = EnvVarGuard::set("OPENAI_API_KEY", "sk-secret-openai");
    let _openai_auth_guard = EnvVarGuard::set("OPENAI_AUTH_TOKEN", "secret-openai-auth");
    let _anthropic_key_guard = EnvVarGuard::set("ANTHROPIC_API_KEY", "secret-anthropic");
    let _authorization_guard = EnvVarGuard::set("AUTHORIZATION", "Bearer secret");
    let _cookie_guard = EnvVarGuard::set("COOKIE", "session=secret");
    let options = CodexAuthProbeOptions::default().with_timeout(Duration::from_secs(5));
    let guard = CodexSetupActionGuard::default();
    let runner = ProcessCodexAuthCommandRunner;

    // When
    let install_receipt =
        install_codex_cli_with_runner(&options, &runner, &guard, Duration::from_secs(5));
    let readiness = probe_codex_provider_auth_with_runner(&options, &runner);
    let login_receipt =
        start_codex_login_with_runner(&options, &runner, &guard, Duration::from_secs(5));
    wait_for_file(
        &capture_dir.join("codex_login_env.txt"),
        Duration::from_secs(5),
    )?;

    // Then
    assert_eq!(install_receipt.status, CodexSetupActionStatus::Installed);
    assert_eq!(readiness.status, CodexAuthStatus::LoggedInUsingChatGpt);
    assert_eq!(login_receipt.status, CodexLoginLaunchStatus::Launched);
    for name in [
        "curl_env.txt",
        "installer_env.txt",
        "codex_status_env.txt",
        "codex_login_env.txt",
    ] {
        assert_forbidden_env_absent(&capture_dir.join(name))?;
    }
    Ok(())
}

#[test]
fn codex_setup_process_runner_uses_structured_curl_shell_stdin_and_reprobes() -> Result<(), String>
{
    // Given
    let _env_lock = lock_env()?;
    let temp_dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let fake_bin = temp_dir.path().join("bin");
    let home_dir = temp_dir.path().join("home");
    let capture_dir = temp_dir.path().join("capture");
    fs::create_dir_all(&fake_bin).map_err(|error| error.to_string())?;
    fs::create_dir_all(&home_dir).map_err(|error| error.to_string())?;
    fs::create_dir_all(&capture_dir).map_err(|error| error.to_string())?;
    let curl_path = fake_bin.join("curl");
    write_executable(
        &curl_path,
        &format!(
            r#"#!/bin/sh
{{
  printf 'argc=%s\n' "$#"
  i=1
  for arg in "$@"; do
    printf 'arg%s=%s\n' "$i" "$arg"
    i=$((i + 1))
  done
}} > {}
cat <<'INSTALLER'
printf '%s\n' "$CODEX_NON_INTERACTIVE" > {}
printf '%s\n' "$CODEX_INSTALL_DIR" > {}
printf '%s|%s\n' "$0" "$#" > {}
mkdir -p "$CODEX_INSTALL_DIR"
cat > "$CODEX_INSTALL_DIR/codex" <<'CODEX'
#!/bin/sh
if [ "$1" = "login" ] && [ "$2" = "status" ]; then
  printf 'Logged in using ChatGPT\n'
  exit 0
fi
exit 0
CODEX
chmod 700 "$CODEX_INSTALL_DIR/codex"
INSTALLER
"#,
            sh_single_quote(&capture_dir.join("curl_args.txt")),
            sh_single_quote(&capture_dir.join("non_interactive.txt")),
            sh_single_quote(&capture_dir.join("install_dir.txt")),
            sh_single_quote(&capture_dir.join("shell_invocation.txt"))
        ),
    )?;
    let test_path = format!("{}:/usr/bin:/bin", fake_bin.display());
    let _path_guard = EnvVarGuard::set("PATH", test_path);
    let _home_guard = EnvVarGuard::set("HOME", home_dir.as_os_str());
    let options = CodexAuthProbeOptions::default().with_timeout(Duration::from_secs(5));
    let guard = CodexSetupActionGuard::default();
    let runner = ProcessCodexAuthCommandRunner;

    // When
    let receipt = install_codex_cli_with_runner(&options, &runner, &guard, Duration::from_secs(5));
    let readiness = probe_codex_provider_auth_with_runner(&options, &runner);

    // Then
    assert_eq!(receipt.status, CodexSetupActionStatus::Installed);
    assert_eq!(readiness.status, CodexAuthStatus::LoggedInUsingChatGpt);
    assert!(readiness.ready);
    assert_eq!(
        fs::read_to_string(capture_dir.join("curl_args.txt"))
            .map_err(|error| error.to_string())?,
        "argc=7\narg1=--fail\narg2=--silent\narg3=--show-error\narg4=--location\narg5=--max-time\narg6=30\narg7=https://chatgpt.com/codex/install.sh\n"
    );
    assert_eq!(
        fs::read_to_string(capture_dir.join("non_interactive.txt"))
            .map_err(|error| error.to_string())?,
        "1\n"
    );
    assert_eq!(
        fs::read_to_string(capture_dir.join("install_dir.txt"))
            .map_err(|error| error.to_string())?,
        format!("{}\n", home_dir.join(".local").join("bin").display())
    );
    let shell_invocation = fs::read_to_string(capture_dir.join("shell_invocation.txt"))
        .map_err(|error| error.to_string())?;
    assert!(
        shell_invocation.ends_with("|0\n"),
        "installer shell should receive bytes on stdin without extra args: {shell_invocation}"
    );
    assert!(
        shell_invocation.contains("sh"),
        "installer should run through /bin/sh: {shell_invocation}"
    );
    Ok(())
}

#[test]
fn codex_setup_process_runner_launches_login_drains_and_reaps_timeout() -> Result<(), String> {
    // Given
    let _env_lock = lock_env()?;
    let temp_dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let capture_dir = temp_dir.path().join("capture");
    fs::create_dir_all(&capture_dir).map_err(|error| error.to_string())?;
    let fixture = CodexAuthFixture::with_script(&format!(
        r#"#!/bin/sh
if [ "$1" = "login" ]; then
  if read unused_input; then
    printf 'stdin-open\n' > {}
  else
    printf 'stdin-null\n' > {}
  fi
  printf '%s\n' "$$" > {}
  i=0
  while [ "$i" -lt 256 ]; do
    printf 'login stdout %s padding padding padding padding\n' "$i"
    printf 'login stderr %s padding padding padding padding\n' "$i" >&2
    i=$((i + 1))
  done
  while true; do :; done
fi
exit 0
"#,
        sh_single_quote(&capture_dir.join("stdin.txt")),
        sh_single_quote(&capture_dir.join("stdin.txt")),
        sh_single_quote(&capture_dir.join("login.pid"))
    ))?;
    let guard = CodexSetupActionGuard::default();
    let runner = ProcessCodexAuthCommandRunner;

    // When
    let receipt =
        start_codex_login_with_runner(&fixture.options, &runner, &guard, Duration::from_secs(2));
    let pid = wait_for_file(&capture_dir.join("login.pid"), Duration::from_secs(5))?
        .trim()
        .parse::<u32>()
        .map_err(|error| error.to_string())?;
    let duplicate =
        start_codex_login_with_runner(&fixture.options, &runner, &guard, Duration::from_secs(2));
    wait_for_process_exit(pid, Duration::from_secs(5))?;

    // Then
    assert_eq!(receipt.status, CodexLoginLaunchStatus::Launched);
    assert_eq!(duplicate.status, CodexLoginLaunchStatus::AlreadyRunning);
    assert_eq!(
        fs::read_to_string(capture_dir.join("stdin.txt")).map_err(|error| error.to_string())?,
        "stdin-null\n"
    );
    Ok(())
}

#[test]
fn codex_auth_probe_reports_timeout() -> Result<(), String> {
    // Given
    let fixture = CodexAuthFixture::new()?;
    let runner = FakeCodexAuthRunner::new(CodexLoginStatusRun::TimedOut);

    // When
    let readiness = probe_codex_provider_auth_with_runner(&fixture.options, &runner);

    // Then
    assert_eq!(readiness.status, CodexAuthStatus::Timeout);
    assert!(!readiness.ready);
    Ok(())
}

#[test]
fn codex_auth_probe_reports_unknown_failure_for_misleading_success_output() -> Result<(), String> {
    // Given
    let fixture = CodexAuthFixture::new()?;
    let runner = FakeCodexAuthRunner::new(CodexLoginStatusRun::Completed(
        CodexAuthCommandOutput::new(Some(1), "Logged in using ChatGPT\n", ""),
    ));

    // When
    let readiness = probe_codex_provider_auth_with_runner(&fixture.options, &runner);

    // Then
    assert_eq!(readiness.status, CodexAuthStatus::UnknownFailure);
    assert!(!readiness.ready);
    Ok(())
}

#[test]
fn codex_auth_probe_reports_unknown_failure_for_malformed_output() -> Result<(), String> {
    // Given
    let fixture = CodexAuthFixture::new()?;
    let runner = FakeCodexAuthRunner::new(CodexLoginStatusRun::Completed(
        CodexAuthCommandOutput::new(Some(0), "{ definitely: not codex login status }", ""),
    ));

    // When
    let readiness = probe_codex_provider_auth_with_runner(&fixture.options, &runner);

    // Then
    assert_eq!(readiness.status, CodexAuthStatus::UnknownFailure);
    assert!(!readiness.ready);
    Ok(())
}

fn sanitized_env_curl_script(capture_dir: &Path) -> String {
    format!(
        r#"#!/bin/sh
{}
cat <<'INSTALLER'
{}
mkdir -p "$CODEX_INSTALL_DIR"
cat > "$CODEX_INSTALL_DIR/codex" <<'CODEX'
#!/bin/sh
if [ "$1" = "login" ] && [ "$2" = "status" ]; then
  {}
  printf 'Logged in using ChatGPT\n'
  exit 0
fi
if [ "$1" = "login" ]; then
  {}
  exit 0
fi
exit 1
CODEX
chmod 700 "$CODEX_INSTALL_DIR/codex"
INSTALLER
"#,
        forbidden_env_capture_block(&capture_dir.join("curl_env.txt")),
        forbidden_env_capture_block(&capture_dir.join("installer_env.txt")),
        forbidden_env_capture_block(&capture_dir.join("codex_status_env.txt")),
        forbidden_env_capture_block(&capture_dir.join("codex_login_env.txt"))
    )
}

fn forbidden_env_capture_block(path: &Path) -> String {
    let mut block = String::from("{\n");
    for key in FORBIDDEN_CHILD_ENV_VARS {
        block.push_str(&format!("  printf '%s=%s\\n' '{key}' \"${{{key}-}}\"\n"));
    }
    block.push_str(&format!("}} > {}\n", sh_single_quote(path)));
    block
}

fn assert_forbidden_env_absent(path: &Path) -> Result<(), String> {
    let contents = fs::read_to_string(path).map_err(|error| error.to_string())?;
    for key in FORBIDDEN_CHILD_ENV_VARS {
        let forbidden_prefix = format!("{key}=");
        for line in contents.lines() {
            if let Some(value) = line.strip_prefix(&forbidden_prefix) {
                assert!(value.is_empty(), "{key} was visible in {}", path.display());
            }
        }
    }
    Ok(())
}

fn sh_single_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\"'\"'"))
}

struct EnvVarGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let previous = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}

struct CodexAuthFixture {
    options: CodexAuthProbeOptions,
    codex_path: PathBuf,
    _temp_dir: tempfile::TempDir,
}

impl CodexAuthFixture {
    fn new() -> Result<Self, String> {
        Self::with_script("#!/bin/sh\nexit 0\n")
    }

    fn with_script(script: &str) -> Result<Self, String> {
        let temp_dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let codex_path = temp_dir.path().join("codex");
        std::fs::write(&codex_path, script).map_err(|error| error.to_string())?;
        mark_executable(&codex_path)?;
        let options = CodexAuthProbeOptions::from_search_paths(vec![temp_dir.path().to_path_buf()]);
        Ok(Self {
            options,
            codex_path,
            _temp_dir: temp_dir,
        })
    }
}

#[cfg(unix)]
fn mark_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = path.metadata().map_err(|error| error.to_string())?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(path, permissions).map_err(|error| error.to_string())
}

#[cfg(not(unix))]
fn mark_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn write_executable(path: &Path, script: &str) -> Result<(), String> {
    fs::write(path, script).map_err(|error| error.to_string())?;
    mark_executable(path)
}

fn wait_for_file(path: &Path, timeout: Duration) -> Result<String, String> {
    let started = Instant::now();
    while started.elapsed() < timeout {
        match fs::read_to_string(path) {
            Ok(contents) => return Ok(contents),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    Err(format!("timed out waiting for {}", path.display()))
}

fn wait_for_process_exit(pid: u32, timeout: Duration) -> Result<(), String> {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if !process_is_alive(pid) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }
    Err(format!("process {pid} was still alive after timeout"))
}

fn process_is_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn lock_env() -> Result<MutexGuard<'static, ()>, String> {
    ENV_LOCK.lock().map_err(|error| error.to_string())
}
