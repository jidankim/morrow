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
            prompt_text: request.prompt().to_owned(),
            timeout: request.timeout(),
            schema_text,
            cwd_entry_count,
        });
        match self.outcomes.borrow_mut().pop() {
            Some(FakeOutcome::WriteOutput(text)) => write_output(request, &text),
            Some(FakeOutcome::ClassifyWeakCalendarFromPrompt) => {
                write_output(request, &weak_calendar_output(request.prompt()))
            }
            Some(FakeOutcome::ClassifyWeakTaskFromPrompt) => {
                write_output(request, &weak_task_output(request.prompt()))
            }
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
    ClassifyWeakCalendarFromPrompt,
    ClassifyWeakTaskFromPrompt,
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
    pub prompt_text: String,
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

fn weak_calendar_output(prompt: &str) -> String {
    if prompt_supports_weak_calendar(prompt) {
        candidate_output(CalibratedCandidate {
            kind: "calendar_event",
            title: "Prompt-calibrated weak calendar",
            confidence_millis: 900,
            normalized_time: "2026-06-26T15:00:00[Asia/Seoul]",
        })
    } else {
        candidate_output(CalibratedCandidate {
            kind: "task_reminder",
            title: "Uncalibrated weak calendar",
            confidence_millis: 500,
            normalized_time: "2026-06-26T15:00:00[Asia/Seoul]",
        })
    }
}

fn weak_task_output(prompt: &str) -> String {
    if prompt_supports_weak_task(prompt) {
        candidate_output(CalibratedCandidate {
            kind: "task_reminder",
            title: "Prompt-calibrated weak task",
            confidence_millis: 760,
            normalized_time: "2026-07-25T23:59:00[Asia/Seoul]",
        })
    } else {
        candidate_output(CalibratedCandidate {
            kind: "calendar_event",
            title: "Uncalibrated weak task",
            confidence_millis: 500,
            normalized_time: "2026-07-25T23:59:00[Asia/Seoul]",
        })
    }
}

struct CalibratedCandidate<'a> {
    kind: &'a str,
    title: &'a str,
    confidence_millis: i64,
    normalized_time: &'a str,
}

fn candidate_output(candidate: CalibratedCandidate<'_>) -> String {
    let kind = candidate.kind;
    let title = candidate.title;
    let confidence_millis = candidate.confidence_millis;
    let normalized_time = candidate.normalized_time;
    format!(
        "{{\"kind\":\"{kind}\",\"title\":\"{title}\",\"confidence_millis\":{confidence_millis},\
         \"normalized_time\":\"{normalized_time}\",\
         \"anchor_evidence_id\":\"evidence://selected/0\",\
         \"evidence_ids\":[\"evidence://selected/0\"]}}"
    )
}

fn prompt_supports_weak_calendar(prompt: &str) -> bool {
    let prompt = prompt.to_ascii_lowercase();
    let examples = ["catch up", "coffee", "sync", "touch base"]
        .iter()
        .filter(|example| prompt.contains(*example))
        .count();
    examples >= 3
        && prompt.contains("calendar_event")
        && prompt_contains_confidence_band(&prompt, 850, 1000)
        && prompt.contains("inferable")
}

fn prompt_supports_weak_task(prompt: &str) -> bool {
    let prompt = prompt.to_ascii_lowercase();
    let examples = [
        "follow up",
        "send by",
        "finish by",
        "complete by",
        "due",
        "deadline",
    ]
    .iter()
    .filter(|example| prompt.contains(*example))
    .count();
    examples >= 4
        && prompt.contains("task_reminder")
        && prompt_contains_confidence_band(&prompt, 700, 850)
}

fn prompt_contains_confidence_band(prompt: &str, lower: i64, upper: i64) -> bool {
    prompt.contains(&lower.to_string()) && prompt.contains(&upper.to_string())
}
