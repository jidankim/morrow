use std::{
    fmt,
    io::{self, Read, Write},
    process::{Child, Command, ExitStatus, Stdio},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use super::request::CodexExecRequest;

const POLL_INTERVAL: Duration = Duration::from_millis(20);

#[derive(Clone, PartialEq, Eq)]
pub struct CodexCommandOutput {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl CodexCommandOutput {
    pub fn new(exit_code: Option<i32>, stdout: &str, stderr: &str) -> Self {
        Self {
            exit_code,
            stdout: stdout.to_owned(),
            stderr: stderr.to_owned(),
        }
    }

    pub const fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }
}

impl fmt::Debug for CodexCommandOutput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CodexCommandOutput")
            .field("exit_code", &self.exit_code)
            .field("stdout", &"<redacted>")
            .field("stderr", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodexExecRun {
    Completed(CodexCommandOutput),
    MissingCli,
    TimedOut,
    FailedToStart,
}

pub trait CodexExecRunner {
    fn run_exec(&self, request: &CodexExecRequest) -> CodexExecRun;
}

impl<R: CodexExecRunner + ?Sized> CodexExecRunner for &R {
    fn run_exec(&self, request: &CodexExecRequest) -> CodexExecRun {
        (*self).run_exec(request)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ProcessCodexExecRunner;

impl CodexExecRunner for ProcessCodexExecRunner {
    fn run_exec(&self, request: &CodexExecRequest) -> CodexExecRun {
        let mut child = match Command::new(request.executable())
            .args(request.args())
            .current_dir(request.cwd())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return CodexExecRun::MissingCli
            }
            Err(_) => return CodexExecRun::FailedToStart,
        };
        run_child(request, &mut child)
    }
}

fn run_child(request: &CodexExecRequest, child: &mut Child) -> CodexExecRun {
    let stdin = match child.stdin.take() {
        Some(stdin) => stdin,
        None => {
            terminate_child(child);
            return CodexExecRun::FailedToStart;
        }
    };
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            terminate_child(child);
            return CodexExecRun::FailedToStart;
        }
    };
    let stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            terminate_child(child);
            return CodexExecRun::FailedToStart;
        }
    };
    let prompt = request.prompt.clone();
    let stdin_writer = thread::spawn(move || write_prompt(stdin, &prompt));
    let stdout_reader = spawn_pipe_reader(stdout);
    let stderr_reader = spawn_pipe_reader(stderr);
    let started = Instant::now();
    while started.elapsed() < request.timeout() {
        match child.try_wait() {
            Ok(Some(status)) => {
                return complete_child(status, stdin_writer, stdout_reader, stderr_reader);
            }
            Ok(None) => thread::sleep(POLL_INTERVAL),
            Err(_) => return CodexExecRun::FailedToStart,
        }
    }
    terminate_child(child);
    join_stdin_writer(stdin_writer);
    drop(join_pipe_reader(stdout_reader));
    drop(join_pipe_reader(stderr_reader));
    CodexExecRun::TimedOut
}

fn write_prompt(mut stdin: impl Write, prompt: &str) -> io::Result<()> {
    stdin.write_all(prompt.as_bytes())?;
    stdin.flush()
}

fn spawn_pipe_reader<R>(mut pipe: R) -> JoinHandle<io::Result<Vec<u8>>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut output = Vec::new();
        pipe.read_to_end(&mut output)?;
        Ok(output)
    })
}

fn complete_child(
    status: ExitStatus,
    stdin_writer: JoinHandle<io::Result<()>>,
    stdout_reader: JoinHandle<io::Result<Vec<u8>>>,
    stderr_reader: JoinHandle<io::Result<Vec<u8>>>,
) -> CodexExecRun {
    let prompt_written = join_stdin_writer(stdin_writer);
    let stdout = match join_pipe_reader(stdout_reader) {
        Ok(output) => output,
        Err(()) => return CodexExecRun::FailedToStart,
    };
    let stderr = match join_pipe_reader(stderr_reader) {
        Ok(output) => output,
        Err(()) => return CodexExecRun::FailedToStart,
    };
    if status.success() && !prompt_written {
        return CodexExecRun::FailedToStart;
    }
    CodexExecRun::Completed(CodexCommandOutput {
        exit_code: status.code(),
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}

fn join_stdin_writer(stdin_writer: JoinHandle<io::Result<()>>) -> bool {
    matches!(stdin_writer.join(), Ok(Ok(())))
}

fn join_pipe_reader(reader: JoinHandle<io::Result<Vec<u8>>>) -> Result<Vec<u8>, ()> {
    match reader.join() {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(_)) | Err(_) => Err(()),
    }
}

fn terminate_child(child: &mut Child) {
    drop(child.kill());
    drop(child.wait());
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        time::{Duration, Instant},
    };

    use super::{CodexExecRun, CodexExecRunner, ProcessCodexExecRunner};
    use crate::native_bridge::codex_provider::request::{codex_exec_args, CodexExecRequest};

    #[test]
    fn codex_process_runner_writes_prompt_to_stdin_and_drains_output() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let executable = dir.path().join("fake-codex");
        let cwd = dir.path().join("cwd");
        let schema_path = dir.path().join("schema.json");
        let output_path = dir.path().join("last-message.json");
        fs::create_dir(&cwd).map_err(|error| error.to_string())?;
        fs::write(&schema_path, "{}").map_err(|error| error.to_string())?;
        fs::write(&executable, fake_codex_script()).map_err(|error| error.to_string())?;
        let mut permissions = fs::metadata(&executable)
            .map_err(|error| error.to_string())?
            .permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&executable, permissions).map_err(|error| error.to_string())?;
        let request = CodexExecRequest {
            executable,
            args: codex_exec_args(&cwd, &schema_path, &output_path),
            cwd,
            schema_path,
            output_path,
            prompt: "PROCESS_PROMPT_SECRET selected_chat_evidence".to_owned(),
            timeout: Duration::from_secs(2),
        };
        let runner = ProcessCodexExecRunner;

        let started = Instant::now();
        let run = runner.run_exec(&request);

        assert!(started.elapsed() < Duration::from_secs(2));
        match run {
            CodexExecRun::Completed(output) => {
                assert_eq!(output.exit_code(), Some(0));
                Ok(())
            }
            other => Err(format!("expected completed fake codex run, got {other:?}")),
        }
    }

    fn fake_codex_script() -> &'static str {
        r#"#!/bin/sh
output=""
previous=""
for arg in "$@"; do
  case "$arg" in
    *PROCESS_PROMPT_SECRET*) exit 5 ;;
  esac
  if [ "$previous" = "--output-last-message" ]; then
    output="$arg"
  fi
  previous="$arg"
done
prompt="$(cat)"
case "$prompt" in
  *PROCESS_PROMPT_SECRET*) ;;
  *) exit 6 ;;
esac
i=0
while [ "$i" -lt 5000 ]; do
  printf '0123456789abcdef0123456789abcdef'
  i=$((i + 1))
done
i=0
while [ "$i" -lt 5000 ]; do
  printf 'fedcba9876543210fedcba9876543210' >&2
  i=$((i + 1))
done
printf '%s' '{"kind":"calendar_event"}' > "$output"
"#
    }
}
