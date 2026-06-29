use std::{
    fmt, io,
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

use super::pipes::PipeReaders;

const LOGIN_STATUS_ARGS: [&str; 2] = ["login", "status"];
const POLL_INTERVAL: Duration = Duration::from_millis(20);

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
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ProcessCodexAuthCommandRunner;

impl CodexAuthCommandRunner for ProcessCodexAuthCommandRunner {
    fn run_login_status(&self, executable: &Path, timeout: Duration) -> CodexLoginStatusRun {
        let mut child = match Command::new(executable)
            .args(LOGIN_STATUS_ARGS)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
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

fn wait_for_login_status(
    child: &mut Child,
    readers: PipeReaders,
    timeout: Duration,
) -> CodexLoginStatusRun {
    let started = Instant::now();
    while started.elapsed() < timeout {
        match child.try_wait() {
            Ok(Some(status)) => {
                return complete_login_status(status, readers);
            }
            Ok(None) => thread::sleep(POLL_INTERVAL),
            Err(_error) => {
                terminate_child_with_readers(child, readers);
                return CodexLoginStatusRun::FailedToStart;
            }
        }
    }
    terminate_child_with_readers(child, readers);
    CodexLoginStatusRun::TimedOut
}

fn complete_login_status(status: ExitStatus, readers: PipeReaders) -> CodexLoginStatusRun {
    let Ok((stdout, stderr)) = readers.join() else {
        return CodexLoginStatusRun::FailedToStart;
    };
    CodexLoginStatusRun::Completed(CodexAuthCommandOutput {
        exit_code: status.code(),
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}

fn terminate_child_with_readers(child: &mut Child, readers: PipeReaders) {
    terminate_child(child);
    drop(readers.join());
}

fn terminate_child(child: &mut Child) {
    drop(child.kill());
    drop(child.wait());
}
