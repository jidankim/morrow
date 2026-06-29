use std::path::PathBuf;

use morrow_diagnostics::export::phoenix::{
    default_phoenix_export_path, export_phoenix_payload, PhoenixExportConfig, PhoenixExportError,
};

fn main() -> Result<(), CliError> {
    let args = CliArgs::parse(std::env::args().skip(1))?;
    let receipt = export_phoenix_payload(&PhoenixExportConfig {
        input_path: &args.input_path,
        output_path: &args.output_path,
    })?;
    println!("phoenix payload: {}", receipt.output_path.display());
    println!("span count: {}", receipt.span_count);
    println!("skipped corrupt lines: {}", receipt.skipped_corrupt_lines);
    println!("payload bytes: {}", receipt.payload_bytes);
    println!("network calls: none");
    Ok(())
}

struct CliArgs {
    input_path: PathBuf,
    output_path: PathBuf,
}

impl CliArgs {
    fn parse<I>(args: I) -> Result<Self, CliError>
    where
        I: Iterator<Item = String>,
    {
        let mut input_path = None;
        let mut output_path = None;
        let mut app_data_dir = None;
        let mut args = args;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--input" => input_path = Some(next_path(&mut args, "--input")?),
                "--out" => output_path = Some(next_path(&mut args, "--out")?),
                "--app-data" => app_data_dir = Some(next_path(&mut args, "--app-data")?),
                "--help" | "-h" => return Err(CliError::Usage),
                flag => {
                    return Err(CliError::UnknownFlag {
                        flag: flag.to_owned(),
                    });
                }
            }
        }
        let input_path = input_path.ok_or(CliError::MissingFlag { flag: "--input" })?;
        let output_path = match (output_path, app_data_dir) {
            (Some(path), _) => path,
            (None, Some(app_data)) => default_phoenix_export_path(&app_data),
            (None, None) => return Err(CliError::MissingOutput),
        };
        Ok(Self {
            input_path,
            output_path,
        })
    }
}

fn next_path<I>(args: &mut I, flag: &'static str) -> Result<PathBuf, CliError>
where
    I: Iterator<Item = String>,
{
    args.next()
        .map(PathBuf::from)
        .ok_or(CliError::MissingValue { flag })
}

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("usage: export_phoenix --input <trace-jsonl-or-dir> (--out <payload-json> | --app-data <app-data-dir>)")]
    Usage,
    #[error("missing required flag {flag}")]
    MissingFlag { flag: &'static str },
    #[error("missing value for {flag}")]
    MissingValue { flag: &'static str },
    #[error("unknown flag {flag}")]
    UnknownFlag { flag: String },
    #[error("missing output; pass --out or --app-data")]
    MissingOutput,
    #[error("phoenix export failed")]
    Export(#[from] PhoenixExportError),
}
