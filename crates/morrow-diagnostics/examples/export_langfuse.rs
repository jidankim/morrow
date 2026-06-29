use std::env;
use std::path::PathBuf;

use morrow_diagnostics::export::write_trace_export_json_file;
use morrow_diagnostics::{
    prove_langfuse_backend_absent, write_langfuse_payload, LangfuseBackendConfig,
    LangfuseExportError,
};

fn main() -> Result<(), CliError> {
    let args = Args::parse(env::args().skip(1))?;
    if args.prove_backend_absent {
        let config = LangfuseBackendConfig {
            endpoint: args.endpoint,
            public_key: args.public_key,
            secret_key: args.secret_key,
        };
        match prove_langfuse_backend_absent(&config) {
            Err(LangfuseExportError::BackendUnavailable { receipt }) => {
                write_trace_export_json_file(&args.input, &args.out, &receipt)?;
                return Err(CliError::BackendAbsent);
            }
            Err(error) => return Err(CliError::Export(error)),
            Ok(receipt) => {
                write_trace_export_json_file(&args.input, &args.out, &receipt)?;
                return Ok(());
            }
        }
    }
    let payload = write_langfuse_payload(&args.input, &args.out)?;
    println!(
        "wrote {} langfuse observations to {} (skipped_corrupt_lines={})",
        payload.export.records_read,
        args.out.display(),
        payload.export.skipped_corrupt_lines
    );
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct Args {
    input: PathBuf,
    out: PathBuf,
    endpoint: Option<String>,
    public_key: Option<String>,
    secret_key: Option<String>,
    prove_backend_absent: bool,
}

impl Args {
    fn parse<I>(args: I) -> Result<Self, CliError>
    where
        I: IntoIterator<Item = String>,
    {
        let mut input = None;
        let mut out = None;
        let mut endpoint = None;
        let mut public_key = None;
        let mut secret_key = None;
        let mut prove_backend_absent = false;
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--input" => input = Some(next_value(&mut iter, "--input")?),
                "--out" => out = Some(next_value(&mut iter, "--out")?),
                "--endpoint" => endpoint = Some(next_value(&mut iter, "--endpoint")?),
                "--public-key" => public_key = Some(next_value(&mut iter, "--public-key")?),
                "--secret-key" => secret_key = Some(next_value(&mut iter, "--secret-key")?),
                "--prove-backend-absent" => prove_backend_absent = true,
                _ => return Err(CliError::UnknownArg(arg)),
            }
        }
        Ok(Self {
            input: input.ok_or(CliError::MissingArg("--input"))?.into(),
            out: out.ok_or(CliError::MissingArg("--out"))?.into(),
            endpoint,
            public_key,
            secret_key,
            prove_backend_absent,
        })
    }
}

fn next_value<I>(iter: &mut I, flag: &'static str) -> Result<String, CliError>
where
    I: Iterator<Item = String>,
{
    iter.next().ok_or(CliError::MissingArg(flag))
}

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("missing required argument {0}")]
    MissingArg(&'static str),
    #[error("unknown argument {0}")]
    UnknownArg(String),
    #[error("langfuse backend absent; receipt written")]
    BackendAbsent,
    #[error("langfuse export failed")]
    Export(#[from] LangfuseExportError),
    #[error("langfuse receipt write failed")]
    Write(#[from] morrow_diagnostics::ExportReadError),
}
