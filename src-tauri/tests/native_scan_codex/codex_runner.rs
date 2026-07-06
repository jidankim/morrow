use std::{cell::RefCell, fs};

use morrow_lib::native_bridge::{
    CodexCommandOutput, CodexExecRequest, CodexExecRun, CodexExecRunner,
};

#[derive(Debug)]
pub struct RecordingCodexRunner {
    outcomes: RefCell<Vec<FakeCodexOutcome>>,
    run_count: RefCell<usize>,
}

impl RecordingCodexRunner {
    pub fn new(outcomes: Vec<FakeCodexOutcome>) -> Self {
        Self {
            outcomes: RefCell::new(outcomes),
            run_count: RefCell::new(0),
        }
    }

    pub fn run_count(&self) -> usize {
        *self.run_count.borrow()
    }
}

impl CodexExecRunner for RecordingCodexRunner {
    fn run_exec(&self, request: &CodexExecRequest) -> CodexExecRun {
        *self.run_count.borrow_mut() += 1;
        match self.outcomes.borrow_mut().pop() {
            Some(FakeCodexOutcome::WriteOutput(text)) => {
                match fs::write(request.output_path(), text) {
                    Ok(()) => CodexExecRun::Completed(CodexCommandOutput::new(Some(0), "", "")),
                    Err(error) => CodexExecRun::Completed(CodexCommandOutput::new(
                        Some(1),
                        "",
                        &error.to_string(),
                    )),
                }
            }
            Some(FakeCodexOutcome::TimedOut) => CodexExecRun::TimedOut,
            None => CodexExecRun::FailedToStart,
        }
    }
}

#[derive(Debug, Clone)]
pub enum FakeCodexOutcome {
    WriteOutput(String),
    TimedOut,
}
