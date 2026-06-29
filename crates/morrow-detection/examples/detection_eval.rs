use std::error::Error;
use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use morrow_diagnostics::validate_trace_record_privacy;

#[path = "detection_eval/support.rs"]
mod support;

use support::{run_eval, EvalRun};

struct Args {
    fixtures: PathBuf,
    out: PathBuf,
    trace_out: PathBuf,
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<bool, Box<dyn Error>> {
    let args = Args::parse()?;
    let eval = run_eval(&args.fixtures)?;
    write_trace_jsonl(&args.trace_out, &eval)?;
    write_summary(&args.out, &eval)?;
    if eval.passed {
        println!(
            "PASS detection_eval scenarios={} traces={}",
            eval.summary.scenario_totals.total,
            eval.records.len()
        );
    } else {
        println!(
            "FAIL detection_eval scenarios_failed={} mismatches={}",
            eval.summary.scenario_totals.failed, eval.summary.mismatch_count
        );
    }
    Ok(eval.passed)
}

impl Args {
    fn parse() -> Result<Self, Box<dyn Error>> {
        let mut fixtures = None;
        let mut out = None;
        let mut trace_out = None;
        let mut args = std::env::args().skip(1);
        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--fixtures" => fixtures = Some(next_path(&mut args, "--fixtures")?),
                "--out" => out = Some(next_path(&mut args, "--out")?),
                "--trace-out" => trace_out = Some(next_path(&mut args, "--trace-out")?),
                "--help" | "-h" => return Err(usage().into()),
                _ => return Err(format!("unknown argument: {flag}\n{}", usage()).into()),
            }
        }
        Ok(Self {
            fixtures: fixtures.ok_or_else(usage)?,
            out: out.ok_or_else(usage)?,
            trace_out: trace_out.ok_or_else(usage)?,
        })
    }
}

fn next_path<I>(args: &mut I, flag: &'static str) -> Result<PathBuf, Box<dyn Error>>
where
    I: Iterator<Item = String>,
{
    let value = args
        .next()
        .ok_or_else(|| format!("{flag} requires a path\n{}", usage()))?;
    Ok(PathBuf::from(value))
}

fn usage() -> String {
    "usage: detection_eval --fixtures <path> --out <path> --trace-out <path>".to_owned()
}

fn write_trace_jsonl(path: &Path, eval: &EvalRun) -> Result<(), Box<dyn Error>> {
    ensure_parent(path)?;
    let mut file = File::create(path)?;
    for record in &eval.records {
        validate_trace_record_privacy(record)?;
        writeln!(file, "{}", serde_json::to_string(record)?)?;
    }
    Ok(())
}

fn write_summary(path: &Path, eval: &EvalRun) -> Result<(), Box<dyn Error>> {
    ensure_parent(path)?;
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, &eval.summary)?;
    Ok(())
}

fn ensure_parent(path: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}
