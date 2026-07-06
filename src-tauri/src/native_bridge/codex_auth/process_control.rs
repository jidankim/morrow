use std::{
    process::{Child, ExitStatus},
    thread,
    time::{Duration, Instant},
};

use super::{
    pipes::PipeReaders,
    process::{CodexAuthCommandOutput, CodexLoginStatusRun},
    setup::CodexLoginRunningGuard,
};

const POLL_INTERVAL: Duration = Duration::from_millis(20);
const READER_JOIN_TIMEOUT: Duration = Duration::from_millis(500);

pub(super) enum ProcessRun {
    Completed(RawProcessOutput),
    MissingExecutable,
    TimedOut(CodexAuthCommandOutput),
    FailedToStart(CodexAuthCommandOutput),
}

pub(super) struct RawProcessOutput {
    exit_code: Option<i32>,
    pub(super) stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl RawProcessOutput {
    pub(super) fn is_success(&self) -> bool {
        self.exit_code == Some(0)
    }
}

impl From<RawProcessOutput> for CodexAuthCommandOutput {
    fn from(output: RawProcessOutput) -> Self {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        Self::new(output.exit_code, &stdout, &stderr)
    }
}

pub(super) fn wait_for_process_output(child: &mut Child, timeout: Duration) -> ProcessRun {
    let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
        terminate_child(child);
        return ProcessRun::FailedToStart(CodexAuthCommandOutput::new(None, "", ""));
    };
    let readers = PipeReaders::spawn(stdout, stderr);
    let started = Instant::now();
    while started.elapsed() < timeout {
        match child.try_wait() {
            Ok(Some(status)) => return complete_process_output(status, readers),
            Ok(None) => thread::sleep(POLL_INTERVAL),
            Err(_error) => {
                terminate_child(child);
                drop(readers);
                return ProcessRun::FailedToStart(empty_command_output());
            }
        }
    }
    terminate_child(child);
    drop(readers);
    ProcessRun::TimedOut(empty_command_output())
}

fn complete_process_output(status: ExitStatus, readers: PipeReaders) -> ProcessRun {
    let Ok((stdout, stderr)) = readers.join(READER_JOIN_TIMEOUT) else {
        return ProcessRun::FailedToStart(empty_command_output());
    };
    ProcessRun::Completed(RawProcessOutput {
        exit_code: status.code(),
        stdout,
        stderr,
    })
}

pub(super) fn supervise_login_child(
    mut child: Child,
    timeout: Duration,
    _running: CodexLoginRunningGuard,
) {
    let started = Instant::now();
    while started.elapsed() < timeout {
        match child.try_wait() {
            Ok(Some(_status)) => return,
            Ok(None) => thread::sleep(POLL_INTERVAL),
            Err(_error) => {
                terminate_child(&mut child);
                return;
            }
        }
    }
    terminate_child(&mut child);
}

pub(super) fn wait_for_login_status(
    child: &mut Child,
    readers: PipeReaders,
    timeout: Duration,
) -> CodexLoginStatusRun {
    let started = Instant::now();
    while started.elapsed() < timeout {
        match child.try_wait() {
            Ok(Some(status)) => return complete_login_status(status, readers),
            Ok(None) => thread::sleep(POLL_INTERVAL),
            Err(_error) => {
                terminate_child(child);
                drop(readers);
                return CodexLoginStatusRun::FailedToStart;
            }
        }
    }
    terminate_child(child);
    drop(readers);
    CodexLoginStatusRun::TimedOut
}

fn complete_login_status(status: ExitStatus, readers: PipeReaders) -> CodexLoginStatusRun {
    let Ok((stdout, stderr)) = readers.join(READER_JOIN_TIMEOUT) else {
        return CodexLoginStatusRun::FailedToStart;
    };
    let stdout = String::from_utf8_lossy(&stdout);
    let stderr = String::from_utf8_lossy(&stderr);
    CodexLoginStatusRun::Completed(CodexAuthCommandOutput::new(status.code(), &stdout, &stderr))
}

fn empty_command_output() -> CodexAuthCommandOutput {
    CodexAuthCommandOutput::new(None, "", "")
}

pub(super) fn terminate_child(child: &mut Child) {
    drop(child.kill());
    drop(child.wait());
}
