use std::fs::{self, File};
use std::io::{BufRead as _, BufReader};
use std::path::{Component, Path, PathBuf};

use crate::sink::parse_trace_record_line;
use crate::{TraceReadResult, TraceReader, TraceSinkError};

pub mod langfuse;
pub mod phoenix;

pub use langfuse::{
    export_langfuse_payload, prove_langfuse_backend_absent, write_langfuse_payload,
    LangfuseBackendConfig, LangfuseBackendReceipt, LangfuseBackendStatus, LangfuseExportError,
    LangfuseExportReceipt, LangfusePayload,
};
pub use phoenix::{
    default_phoenix_export_path, export_phoenix_payload, PhoenixExportConfig, PhoenixExportError,
    PhoenixExportReceipt,
};

pub fn read_trace_input(path: &Path) -> Result<TraceReadResult, ExportReadError> {
    if path.is_dir() {
        return Ok(TraceReader::new(path).read_all()?);
    }
    read_trace_file(path)
}

pub fn write_json_file<T>(path: &Path, value: &T) -> Result<(), ExportReadError>
where
    T: serde::Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, value)?;
    Ok(())
}

pub fn write_trace_export_json_file<T>(
    input: &Path,
    output: &Path,
    value: &T,
) -> Result<(), ExportReadError>
where
    T: serde::Serialize,
{
    ensure_distinct_input_output(input, output)?;
    write_json_file(output, value)
}

pub fn ensure_distinct_input_output(input: &Path, output: &Path) -> Result<(), ExportReadError> {
    let input_path = comparable_path(input)?;
    let output_path = comparable_path(output)?;
    if input_path == output_path {
        return Err(ExportReadError::OutputWouldOverwriteInput {
            input: input.to_path_buf(),
            output: output.to_path_buf(),
        });
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum ExportReadError {
    #[error("trace export io failed")]
    Io(#[from] std::io::Error),
    #[error("trace export serialization failed")]
    Serialization(#[from] serde_json::Error),
    #[error("trace reader failed")]
    TraceSink(#[from] TraceSinkError),
    #[error(
        "trace export output would overwrite input trace file: input={input}, output={output}",
        input = .input.display(),
        output = .output.display()
    )]
    OutputWouldOverwriteInput { input: PathBuf, output: PathBuf },
}

fn read_trace_file(path: &Path) -> Result<TraceReadResult, ExportReadError> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut line = Vec::new();
    let mut records = Vec::new();
    let mut skipped_lines = 0_usize;
    loop {
        line.clear();
        if reader.read_until(b'\n', &mut line)? == 0 {
            break;
        }
        let Ok(line_text) = std::str::from_utf8(&line) else {
            skipped_lines += 1;
            continue;
        };
        match parse_trace_record_line(line_text) {
            Ok(record) => records.push(record),
            Err(_) => skipped_lines += 1,
        }
    }
    Ok(TraceReadResult {
        records,
        skipped_lines,
    })
}

fn comparable_path(path: &Path) -> Result<PathBuf, std::io::Error> {
    if let Ok(canonical) = fs::canonicalize(path) {
        return Ok(normalize_path(&canonical));
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    if let (Some(parent), Some(file_name)) = (absolute.parent(), absolute.file_name()) {
        if let Ok(parent) = fs::canonicalize(parent) {
            return Ok(normalize_path(&parent.join(file_name)));
        }
    }
    Ok(normalize_path(&absolute))
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new("/")),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push("..");
                }
            }
            Component::Normal(segment) => normalized.push(segment),
        }
    }
    normalized
}
