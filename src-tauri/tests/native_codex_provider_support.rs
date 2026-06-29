use std::{cell::RefCell, fs, path::PathBuf, time::Duration};

use morrow_lib::native_bridge::{
    CodexCommandOutput, CodexExecRequest, CodexExecRun, CodexExecRunner,
};

#[derive(Debug)]
pub struct FakeCodexRunner {
    outcomes: RefCell<Vec<FakeOutcome>>,
    observations: RefCell<Vec<RunObservation>>,
}

impl FakeCodexRunner {
    pub fn new(outcomes: Vec<FakeOutcome>) -> Self {
        Self {
            outcomes: RefCell::new(outcomes),
            observations: RefCell::new(Vec::new()),
        }
    }

    pub fn only_observation(&self) -> Result<RunObservation, String> {
        let observations = self.observations.borrow();
        assert_eq!(observations.len(), 1);
        observations
            .first()
            .cloned()
            .ok_or_else(|| "missing codex run observation".to_owned())
    }

    pub fn observation_count(&self) -> usize {
        self.observations.borrow().len()
    }
}

impl CodexExecRunner for FakeCodexRunner {
    fn run_exec(&self, request: &CodexExecRequest) -> CodexExecRun {
        let schema_text =
            fs::read_to_string(request.schema_path()).unwrap_or_else(|error| error.to_string());
        let cwd_entry_count = fs::read_dir(request.cwd())
            .map(|entries| entries.count())
            .unwrap_or(usize::MAX);
        self.observations.borrow_mut().push(RunObservation {
            executable: request.executable().to_path_buf(),
            args: request.args().to_vec(),
            cwd: request.cwd().to_path_buf(),
            schema_path: request.schema_path().to_path_buf(),
            output_path: request.output_path().to_path_buf(),
            timeout: request.timeout(),
            schema_text,
            cwd_entry_count,
        });
        match self.outcomes.borrow_mut().pop() {
            Some(FakeOutcome::WriteOutput(text)) => write_output(request, &text),
            Some(FakeOutcome::Completed(output)) => CodexExecRun::Completed(output),
            Some(FakeOutcome::MissingCli) => CodexExecRun::MissingCli,
            Some(FakeOutcome::Timeout) => CodexExecRun::TimedOut,
            None => CodexExecRun::FailedToStart,
        }
    }
}

#[derive(Debug, Clone)]
pub enum FakeOutcome {
    WriteOutput(String),
    Completed(CodexCommandOutput),
    MissingCli,
    Timeout,
}

#[derive(Debug, Clone)]
pub struct RunObservation {
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub schema_path: PathBuf,
    pub output_path: PathBuf,
    pub timeout: Duration,
    pub schema_text: String,
    pub cwd_entry_count: usize,
}

fn write_output(request: &CodexExecRequest, text: &str) -> CodexExecRun {
    if let Err(error) = fs::write(request.output_path(), text) {
        return CodexExecRun::Completed(CodexCommandOutput::new(Some(1), "", &error.to_string()));
    }
    CodexExecRun::Completed(CodexCommandOutput::new(Some(0), "", ""))
}
