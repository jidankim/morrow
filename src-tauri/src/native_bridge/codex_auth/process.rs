use std::{
    env,
    ffi::OsStr,
    fmt, io,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use super::pipes::PipeReaders;
use super::process_control::{
    supervise_login_child, terminate_child, wait_for_login_status, wait_for_process_output,
    ProcessRun,
};
use super::setup::{CodexLoginLaunchRun, CodexLoginRunningGuard, CodexSetupActionRun};

const CURL_INSTALL_ARGS: [&str; 7] = [
    "--fail",
    "--silent",
    "--show-error",
    "--location",
    "--max-time",
    "30",
    "https://chatgpt.com/codex/install.sh",
];
const INSTALL_DIR_SUFFIX: [&str; 2] = [".local", "bin"];
const LOGIN_ARGS: [&str; 1] = ["login"];
const LOGIN_STATUS_ARGS: [&str; 2] = ["login", "status"];

#[derive(Clone, PartialEq, Eq)]
pub struct CodexAuthCommandOutput {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl CodexAuthCommandOutput {
    pub fn new(exit_code: Option<i32>, stdout: &str, stderr: &str) -> Self {
        Self {
            exit_code,
            stdout: stdout.to_owned(),
            stderr: stderr.to_owned(),
        }
    }

    pub(super) fn has_output(&self) -> bool {
        !self.stdout.is_empty() || !self.stderr.is_empty()
    }

    pub(super) fn is_success(&self) -> bool {
        self.exit_code == Some(0)
    }

    pub(super) fn combined_output(&self) -> String {
        format!("{}\n{}", self.stdout, self.stderr)
    }
}

impl fmt::Debug for CodexAuthCommandOutput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "CodexAuthCommandOutput {{ exit_code: {:?}, stdout: \"<redacted>\", stderr: \"<redacted>\" }}",
            self.exit_code
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodexLoginStatusRun {
    Completed(CodexAuthCommandOutput),
    MissingCli,
    TimedOut,
    FailedToStart,
}

pub trait CodexAuthCommandRunner {
    fn run_login_status(&self, executable: &Path, timeout: Duration) -> CodexLoginStatusRun;

    fn run_codex_install(&self, _timeout: Duration) -> CodexSetupActionRun {
        CodexSetupActionRun::Unsupported
    }

    fn launch_codex_login(&self, _executable: &Path, _timeout: Duration) -> CodexLoginLaunchRun {
        CodexLoginLaunchRun::FailedToStart(CodexAuthCommandOutput::new(None, "", ""))
    }

    fn launch_codex_login_with_guard(
        &self,
        executable: &Path,
        timeout: Duration,
        running: CodexLoginRunningGuard,
    ) -> CodexLoginLaunchRun {
        let run = self.launch_codex_login(executable, timeout);
        drop(running);
        run
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ProcessCodexAuthCommandRunner;

impl CodexAuthCommandRunner for ProcessCodexAuthCommandRunner {
    fn run_codex_install(&self, timeout: Duration) -> CodexSetupActionRun {
        let curl_output = match run_curl_installer_download(timeout) {
            ProcessRun::Completed(output) if output.is_success() => output,
            ProcessRun::Completed(output) => return CodexSetupActionRun::CurlFailed(output.into()),
            ProcessRun::MissingExecutable => return CodexSetupActionRun::MissingCurl,
            ProcessRun::TimedOut(output) => return CodexSetupActionRun::TimedOut(output),
            ProcessRun::FailedToStart(output) => return CodexSetupActionRun::Failed(output),
        };
        let Some(install_dir) = codex_install_dir() else {
            return CodexSetupActionRun::Failed(CodexAuthCommandOutput::new(None, "", ""));
        };
        match run_installer_shell(curl_output.stdout, &install_dir, timeout) {
            ProcessRun::Completed(output) if output.is_success() => {
                CodexSetupActionRun::Installed(output.into())
            }
            ProcessRun::Completed(output) => CodexSetupActionRun::InstallerFailed(output.into()),
            ProcessRun::MissingExecutable | ProcessRun::FailedToStart(_) => {
                CodexSetupActionRun::Failed(CodexAuthCommandOutput::new(None, "", ""))
            }
            ProcessRun::TimedOut(output) => CodexSetupActionRun::TimedOut(output),
        }
    }

    fn launch_codex_login_with_guard(
        &self,
        executable: &Path,
        timeout: Duration,
        running: CodexLoginRunningGuard,
    ) -> CodexLoginLaunchRun {
        let mut command = sanitized_command(executable);
        command
            .args(LOGIN_ARGS)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let child = match command.spawn() {
            Ok(child) => child,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                drop(running);
                return CodexLoginLaunchRun::MissingCli;
            }
            Err(_error) => {
                drop(running);
                return CodexLoginLaunchRun::FailedToStart(CodexAuthCommandOutput::new(
                    None, "", "",
                ));
            }
        };
        thread::spawn(move || supervise_login_child(child, timeout, running));
        CodexLoginLaunchRun::Launched
    }

    fn run_login_status(&self, executable: &Path, timeout: Duration) -> CodexLoginStatusRun {
        let mut command = sanitized_command(executable);
        command
            .args(LOGIN_STATUS_ARGS)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return CodexLoginStatusRun::MissingCli;
            }
            Err(_error) => return CodexLoginStatusRun::FailedToStart,
        };
        let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
            terminate_child(&mut child);
            return CodexLoginStatusRun::FailedToStart;
        };
        let readers = PipeReaders::spawn(stdout, stderr);
        wait_for_login_status(&mut child, readers, timeout)
    }
}

fn run_curl_installer_download(timeout: Duration) -> ProcessRun {
    let mut command = sanitized_command("curl");
    command
        .args(CURL_INSTALL_ARGS)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return ProcessRun::MissingExecutable;
        }
        Err(_error) => {
            return ProcessRun::FailedToStart(CodexAuthCommandOutput::new(None, "", ""));
        }
    };
    wait_for_process_output(&mut child, timeout)
}

fn run_installer_shell(installer: Vec<u8>, install_dir: &Path, timeout: Duration) -> ProcessRun {
    let mut command = sanitized_command("/bin/sh");
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("CODEX_NON_INTERACTIVE", "1")
        .env("CODEX_INSTALL_DIR", install_dir);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_error) => {
            return ProcessRun::FailedToStart(CodexAuthCommandOutput::new(None, "", ""));
        }
    };
    let Some(mut stdin) = child.stdin.take() else {
        terminate_child(&mut child);
        return ProcessRun::FailedToStart(CodexAuthCommandOutput::new(None, "", ""));
    };
    let writer = thread::spawn(move || stdin.write_all(&installer));
    let output = wait_for_process_output(&mut child, timeout);
    drop(writer.join());
    output
}

fn sanitized_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    command.env_clear();
    for key in [
        "HOME",
        "PATH",
        "TMPDIR",
        "TMP",
        "TEMP",
        "LANG",
        "LC_ALL",
        "LC_CTYPE",
        "LC_MESSAGES",
        "LC_COLLATE",
        "LC_TIME",
        "LC_NUMERIC",
        "LC_MONETARY",
    ] {
        if let Some(value) = env::var_os(key).filter(|value| !value.is_empty()) {
            command.env(key, value);
        }
    }
    command
}

fn codex_install_dir() -> Option<PathBuf> {
    let home = env::var_os("HOME").filter(|home| !home.is_empty())?;
    let mut install_dir = PathBuf::from(home);
    for suffix in INSTALL_DIR_SUFFIX {
        install_dir.push(suffix);
    }
    Some(install_dir)
}
