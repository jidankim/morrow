use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::trace::TraceRecord;

use super::{
    diagnostics_trace_dir, JsonlTraceSink, JsonlTraceSinkConfig, TraceReader, TraceSinkError,
    DEFAULT_RETENTION,
};

#[test]
fn reader_counts_corrupted_lines() -> Result<(), TestError> {
    // Given: one JSONL trace file with a valid trace, a corrupted line, and another trace.
    let traces_dir = unique_temp_dir("reader-counts-corrupted-lines")?;
    let trace_file = traces_dir.join("trace-00000000000000000001-0.jsonl");
    fs::write(
        &trace_file,
        format!(
            "{}not-json\n{}",
            include_str!("../../fixtures/trace_v1_snapshot.jsonl"),
            include_str!("../../fixtures/trace_v1_snapshot.jsonl"),
        ),
    )?;

    // When: the reader loads records from the trace directory.
    let result = TraceReader::new(&traces_dir).read_all()?;

    // Then: valid records are returned and the corrupted line is counted.
    assert_eq!(result.records.len(), 2);
    assert_eq!(result.skipped_lines, 1);
    Ok(())
}

#[test]
fn reader_counts_invalid_utf8_corrupted_lines() -> Result<(), TestError> {
    // Given: one JSONL trace file with valid records around a byte-corrupted line.
    let traces_dir = unique_temp_dir("reader-counts-invalid-utf8")?;
    let trace_file = traces_dir.join("trace-00000000000000000001-0.jsonl");
    let valid_record = include_str!("../../fixtures/trace_v1_snapshot.jsonl");
    let mut content = Vec::new();
    content.extend_from_slice(valid_record.as_bytes());
    content.extend_from_slice(&[0xff, 0xfe, b'\n']);
    content.extend_from_slice(valid_record.as_bytes());
    fs::write(&trace_file, content)?;

    // When: the reader loads records from the trace directory.
    let result = TraceReader::new(&traces_dir).read_all()?;

    // Then: valid records are returned and the byte-corrupted line is counted.
    assert_eq!(result.records.len(), 2);
    assert_eq!(result.skipped_lines, 1);
    Ok(())
}

#[test]
fn sink_ignores_non_trace_jsonl_files() -> Result<(), TestError> {
    // Given: a stray JSONL file that sorts after sink-owned trace files.
    let app_data = TestAppData::new("ignore-stray-jsonl")?;
    let sink = JsonlTraceSink::new(app_data.path())?;
    let first_path = sink.append(&TraceRecord::sample_v1())?;
    let stray_file = sink.traces_dir().join("zzz.jsonl");
    fs::write(&stray_file, "{}\n")?;

    // When: another trace record is appended and the reader loads all traces.
    let second_path = sink.append(&TraceRecord::sample_v1())?;
    let result = TraceReader::new(sink.traces_dir()).read_all()?;

    // Then: the stray file is not selected for append and is not read.
    assert_eq!(first_path, second_path);
    assert_eq!(fs::read_to_string(&stray_file)?, "{}\n");
    assert_eq!(result.records.len(), 2);
    assert_eq!(result.skipped_lines, 0);
    Ok(())
}

#[test]
fn jsonl_sink_appends_trace_record() -> Result<(), TestError> {
    // Given: a sink rooted at a synthetic app data directory.
    let app_data = TestAppData::new("append-trace-record")?;
    let sink = JsonlTraceSink::new(app_data.path())?;
    let record = TraceRecord::sample_v1();

    // When: one trace record is appended.
    let written_path = sink.append(&record)?;

    // Then: the trace is written under diagnostics/traces and can be read back.
    assert!(written_path.starts_with(diagnostics_trace_dir(app_data.path())));
    let result = TraceReader::new(sink.traces_dir()).read_all()?;
    assert_eq!(result.records, vec![record]);
    assert_eq!(result.skipped_lines, 0);
    Ok(())
}

#[test]
fn jsonl_sink_rotates_with_small_test_override() -> Result<(), TestError> {
    // Given: a sink configured with a tiny rotation threshold.
    let app_data = TestAppData::new("rotation")?;
    let sink = JsonlTraceSink::with_config(
        app_data.path(),
        JsonlTraceSinkConfig::new(DEFAULT_RETENTION, 1),
    )?;

    // When: multiple trace records are appended.
    let first = sink.append(&TraceRecord::sample_v1())?;
    let second = sink.append(&TraceRecord::sample_v1())?;
    let third = sink.append(&TraceRecord::sample_v1())?;

    // Then: each append after the first rotates into a new JSONL file.
    assert_ne!(first, second);
    assert_ne!(second, third);
    assert_eq!(trace_file_count(sink.traces_dir())?, 3);
    Ok(())
}

#[test]
fn jsonl_sink_retains_thirty_days_by_default() -> Result<(), TestError> {
    // Given: an old synthetic trace file and a default-configured sink.
    let app_data = TestAppData::new("retention")?;
    let traces_dir = diagnostics_trace_dir(app_data.path());
    fs::create_dir_all(&traces_dir)?;
    fs::write(traces_dir.join("trace-1-0.jsonl"), "{}\n")?;

    // When: the sink starts with default 30-day retention.
    let sink = JsonlTraceSink::new(app_data.path())?;

    // Then: the old JSONL trace file is removed before any append.
    assert_eq!(trace_file_count(sink.traces_dir())?, 0);
    assert_eq!(JsonlTraceSinkConfig::default().retention, DEFAULT_RETENTION);
    Ok(())
}

#[test]
fn jsonl_sink_runs_retention_before_append() -> Result<(), TestError> {
    // Given: a sink with a stale trace file added after startup.
    let app_data = TestAppData::new("retention-before-append")?;
    let sink = JsonlTraceSink::new(app_data.path())?;
    fs::write(sink.traces_dir().join("trace-1-0.jsonl"), "{}\n")?;

    // When: a new trace record is appended.
    sink.append(&TraceRecord::sample_v1())?;

    // Then: only the newly appended trace file remains readable.
    let result = TraceReader::new(sink.traces_dir()).read_all()?;
    assert_eq!(result.records.len(), 1);
    assert_eq!(result.skipped_lines, 0);
    Ok(())
}

#[test]
fn jsonl_sink_serializes_same_process_concurrent_appends() -> Result<(), TestError> {
    // Given: a shared sink used by several same-process writer threads.
    let app_data = TestAppData::new("concurrent-append")?;
    let sink = Arc::new(JsonlTraceSink::new(app_data.path())?);
    let thread_count = 6_usize;
    let records_per_thread = 7_usize;
    let mut handles = Vec::new();
    for _ in 0..thread_count {
        let sink = Arc::clone(&sink);
        handles.push(thread::spawn(move || -> Result<(), String> {
            for _ in 0..records_per_thread {
                sink.append(&TraceRecord::sample_v1())
                    .map_err(|error| error.to_string())?;
            }
            Ok(())
        }));
    }

    // When: all writer threads finish.
    for handle in handles {
        handle
            .join()
            .map_err(|_| TestError::ThreadPanicked)?
            .map_err(TestError::Writer)?;
    }

    // Then: every trace line is readable without corruption.
    let result = TraceReader::new(sink.traces_dir()).read_all()?;
    assert_eq!(result.records.len(), thread_count * records_per_thread);
    assert_eq!(result.skipped_lines, 0);
    Ok(())
}

fn unique_temp_dir(name: &str) -> Result<PathBuf, TestError> {
    let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!("morrow-diagnostics-{name}-{suffix}"));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn trace_file_count(traces_dir: &Path) -> Result<usize, TestError> {
    Ok(fs::read_dir(traces_dir)?.count())
}

struct TestAppData {
    path: PathBuf,
}

impl TestAppData {
    fn new(name: &str) -> Result<Self, TestError> {
        Ok(Self {
            path: unique_temp_dir(name)?,
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestAppData {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[derive(Debug, thiserror::Error)]
enum TestError {
    #[error("test io failed")]
    Io(#[from] std::io::Error),
    #[error("test clock failed")]
    Time(#[from] std::time::SystemTimeError),
    #[error("trace sink failed")]
    Sink(#[from] TraceSinkError),
    #[error("trace writer thread panicked")]
    ThreadPanicked,
    #[error("trace writer failed: {0}")]
    Writer(String),
}
