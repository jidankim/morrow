use std::{
    cell::RefCell,
    ffi::OsString,
    path::{Path, PathBuf},
    time::Duration,
};

use morrow_lib::native_bridge::{
    probe_codex_provider_auth_with_runner, CodexAuthCommandOutput, CodexAuthCommandRunner,
    CodexAuthProbeOptions, CodexAuthStatus, CodexLoginStatusRun, ProcessCodexAuthCommandRunner,
};

#[derive(Debug)]
struct FakeCodexAuthRunner {
    result: CodexLoginStatusRun,
    calls: RefCell<Vec<(PathBuf, Duration)>>,
}

impl FakeCodexAuthRunner {
    fn new(result: CodexLoginStatusRun) -> Self {
        Self {
            result,
            calls: RefCell::new(Vec::new()),
        }
    }

    fn calls(&self) -> Vec<(PathBuf, Duration)> {
        self.calls.borrow().clone()
    }
}

impl CodexAuthCommandRunner for FakeCodexAuthRunner {
    fn run_login_status(&self, executable: &Path, timeout: Duration) -> CodexLoginStatusRun {
        self.calls
            .borrow_mut()
            .push((executable.to_path_buf(), timeout));
        self.result.clone()
    }
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
