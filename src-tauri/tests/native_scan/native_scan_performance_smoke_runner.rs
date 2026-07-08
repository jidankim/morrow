use std::{cell::Cell, collections::VecDeque, fs};

use morrow_lib::native_bridge::{
    CodexCommandOutput, CodexExecRequest, CodexExecRun, CodexExecRunner,
};

#[derive(Debug)]
pub(super) struct WriteOutputRunner {
    outputs: std::cell::RefCell<VecDeque<String>>,
    calls: Cell<usize>,
}

impl WriteOutputRunner {
    pub(super) fn with_repeated_output(output: String, count: usize) -> Self {
        Self {
            outputs: std::cell::RefCell::new(VecDeque::from(vec![output; count])),
            calls: Cell::new(0),
        }
    }

    pub(super) fn calls(&self) -> usize {
        self.calls.get()
    }
}

impl CodexExecRunner for WriteOutputRunner {
    fn run_exec(&self, request: &CodexExecRequest) -> CodexExecRun {
        self.calls.set(self.calls.get() + 1);
        let Some(output) = self.outputs.borrow_mut().pop_front() else {
            return CodexExecRun::FailedToStart;
        };
        match fs::write(request.output_path(), output) {
            Ok(()) => CodexExecRun::Completed(CodexCommandOutput::new(Some(0), "", "")),
            Err(error) => {
                CodexExecRun::Completed(CodexCommandOutput::new(Some(1), "", &error.to_string()))
            }
        }
    }
}
