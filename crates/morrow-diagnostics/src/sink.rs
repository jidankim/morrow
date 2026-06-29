use std::fs::{self, File, OpenOptions};
use std::io::{BufRead as _, BufReader, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::privacy::{validate_trace_record_privacy, PrivacyError};
use crate::trace::{TraceRecord, TraceRecorder, TraceRecorderError};

const DEFAULT_RETENTION: Duration = Duration::from_secs(30 * 24 * 60 * 60);
const DEFAULT_ROTATION_BYTES: u64 = 10 * 1024 * 1024;

pub fn diagnostics_trace_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("diagnostics").join("traces")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonlTraceSinkConfig {
    pub retention: Duration,
    pub rotation_bytes: u64,
}

impl JsonlTraceSinkConfig {
    pub const fn new(retention: Duration, rotation_bytes: u64) -> Self {
        Self {
            retention,
            rotation_bytes,
        }
    }
}

impl Default for JsonlTraceSinkConfig {
    fn default() -> Self {
        Self::new(DEFAULT_RETENTION, DEFAULT_ROTATION_BYTES)
    }
}

/// v1 is a single-process writer: the Tauri app is the only runtime writer.
#[derive(Debug)]
pub struct JsonlTraceSink {
    traces_dir: PathBuf,
    config: JsonlTraceSinkConfig,
    lock: Mutex<()>,
}

impl JsonlTraceSink {
    pub fn new(app_data_dir: &Path) -> Result<Self, TraceSinkError> {
        Self::with_config(app_data_dir, JsonlTraceSinkConfig::default())
    }

    pub fn with_config(
        app_data_dir: &Path,
        config: JsonlTraceSinkConfig,
    ) -> Result<Self, TraceSinkError> {
        let traces_dir = diagnostics_trace_dir(app_data_dir);
        fs::create_dir_all(&traces_dir)?;
        apply_retention(&traces_dir, config.retention, current_unix_seconds()?)?;
        Ok(Self {
            traces_dir,
            config,
            lock: Mutex::new(()),
        })
    }

    pub fn traces_dir(&self) -> &Path {
        &self.traces_dir
    }

    pub fn append(&self, record: &TraceRecord) -> Result<PathBuf, TraceSinkError> {
        validate_trace_record_privacy(record)?;
        let _guard = self.lock.lock().map_err(|_| TraceSinkError::LockPoisoned)?;
        let now = current_unix_seconds()?;
        apply_retention(&self.traces_dir, self.config.retention, now)?;
        let line = serde_json::to_string(record)?;
        let write_len =
            u64::try_from(line.len() + 1).map_err(|_| TraceSinkError::RecordTooLarge)?;
        let trace_file = self.append_target(write_len, now)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&trace_file)?;
        writeln!(file, "{line}")?;
        Ok(trace_file)
    }

    fn append_target(&self, write_len: u64, now: u64) -> Result<PathBuf, TraceSinkError> {
        let Some(latest) = latest_trace_file(&self.traces_dir)? else {
            return next_trace_file(&self.traces_dir, now);
        };
        let current_len = fs::metadata(&latest)?.len();
        if current_len > 0 && current_len.saturating_add(write_len) > self.config.rotation_bytes {
            return next_trace_file(&self.traces_dir, now);
        }
        Ok(latest)
    }
}

impl TraceRecorder for JsonlTraceSink {
    fn record(&self, record: &TraceRecord) -> Result<(), TraceRecorderError> {
        self.append(record)
            .map(|_| ())
            .map_err(|_| TraceRecorderError::Unavailable)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceReadResult {
    pub records: Vec<TraceRecord>,
    pub skipped_lines: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceReader {
    traces_dir: PathBuf,
}

impl TraceReader {
    pub fn new(traces_dir: &Path) -> Self {
        Self {
            traces_dir: traces_dir.to_path_buf(),
        }
    }

    pub fn read_all(&self) -> Result<TraceReadResult, TraceSinkError> {
        let mut records = Vec::new();
        let mut skipped_lines = 0_usize;
        for trace_file in trace_jsonl_files(&self.traces_dir)? {
            let file = File::open(trace_file)?;
            let mut reader = BufReader::new(file);
            let mut line = Vec::new();
            loop {
                line.clear();
                if reader.read_until(b'\n', &mut line)? == 0 {
                    break;
                }
                let line = match std::str::from_utf8(&line) {
                    Ok(line) => line,
                    Err(_) => {
                        skipped_lines += 1;
                        continue;
                    }
                };
                match parse_trace_record_line(line) {
                    Ok(record) => records.push(record),
                    Err(_) => skipped_lines += 1,
                }
            }
        }
        Ok(TraceReadResult {
            records,
            skipped_lines,
        })
    }
}

pub(crate) fn parse_trace_record_line(line: &str) -> Result<TraceRecord, TraceSinkError> {
    let record = serde_json::from_str::<TraceRecord>(line)?;
    validate_trace_record_privacy(&record)?;
    Ok(record)
}

#[derive(Debug, thiserror::Error)]
pub enum TraceSinkError {
    #[error("trace sink io failed")]
    Io(#[from] std::io::Error),
    #[error("trace serialization failed")]
    Serialization(#[from] serde_json::Error),
    #[error("trace privacy validation failed")]
    Privacy(#[from] PrivacyError),
    #[error("trace sink mutex is poisoned")]
    LockPoisoned,
    #[error("system time is before unix epoch")]
    TimeBeforeEpoch,
    #[error("trace record is too large")]
    RecordTooLarge,
}

fn latest_trace_file(traces_dir: &Path) -> Result<Option<PathBuf>, TraceSinkError> {
    let mut files = trace_jsonl_files(traces_dir)?;
    Ok(files.pop())
}

fn next_trace_file(traces_dir: &Path, now: u64) -> Result<PathBuf, TraceSinkError> {
    let mut sequence = 0_u64;
    loop {
        let path = traces_dir.join(format!("trace-{now}-{sequence}.jsonl"));
        if !path.exists() {
            return Ok(path);
        }
        sequence = sequence
            .checked_add(1)
            .ok_or(TraceSinkError::RecordTooLarge)?;
    }
}

fn apply_retention(traces_dir: &Path, retention: Duration, now: u64) -> Result<(), TraceSinkError> {
    for trace_file in trace_jsonl_files(traces_dir)? {
        let Some(created_at) = trace_file_created_at(&trace_file) else {
            continue;
        };
        if now.saturating_sub(created_at) > retention.as_secs() {
            fs::remove_file(trace_file)?;
        }
    }
    Ok(())
}

fn trace_jsonl_files(traces_dir: &Path) -> Result<Vec<PathBuf>, TraceSinkError> {
    if !traces_dir.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(traces_dir)? {
        let path = entry?.path();
        if let Some(sort_key) = trace_file_key(&path) {
            files.push((sort_key, path));
        }
    }
    files.sort_by_key(|(sort_key, _path)| *sort_key);
    Ok(files.into_iter().map(|(_sort_key, path)| path).collect())
}

fn trace_file_created_at(path: &Path) -> Option<u64> {
    trace_file_key(path).map(|(timestamp, _sequence)| timestamp)
}

fn trace_file_key(path: &Path) -> Option<(u64, u64)> {
    let file_name = path.file_name()?.to_str()?;
    let stem = file_name.strip_prefix("trace-")?.strip_suffix(".jsonl")?;
    let (timestamp, sequence) = stem.split_once('-')?;
    Some((timestamp.parse().ok()?, sequence.parse().ok()?))
}

fn current_unix_seconds() -> Result<u64, TraceSinkError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| TraceSinkError::TimeBeforeEpoch)?
        .as_secs())
}

#[cfg(test)]
mod jsonl_sink;
