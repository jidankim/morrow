use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::validation::validate_text;
use crate::StorageError;

const FIELD_SEPARATOR: &str = "\u{1f}";

#[derive(Debug, Clone)]
pub(crate) struct Sqlite {
    db_path: PathBuf,
}

impl Sqlite {
    pub(crate) fn new(db_path: &Path) -> Self {
        Self {
            db_path: db_path.to_path_buf(),
        }
    }

    pub(crate) fn execute(&self, sql: &str) -> Result<(), StorageError> {
        let output = self.run(sql)?;
        if output.status.success() {
            Ok(())
        } else {
            Err(sqlite_error(&output.stderr))
        }
    }

    pub(crate) fn query_scalar_i64(&self, sql: &str) -> Result<i64, StorageError> {
        let values = self.query_first_column(sql)?;
        let raw = values.first().ok_or_else(|| StorageError::Sqlite {
            message: "query returned no rows".to_owned(),
        })?;
        raw.parse::<i64>().map_err(|err| StorageError::Sqlite {
            message: format!("expected integer result: {err}"),
        })
    }

    pub(crate) fn query_first_column(&self, sql: &str) -> Result<Vec<String>, StorageError> {
        self.query_rows(sql).map(|rows| {
            rows.into_iter()
                .filter_map(|row| row.into_iter().next())
                .collect()
        })
    }

    pub(crate) fn query_rows(&self, sql: &str) -> Result<Vec<Vec<String>>, StorageError> {
        let output = self.run(sql)?;
        if !output.status.success() {
            return Err(sqlite_error(&output.stderr));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| line.split(FIELD_SEPARATOR).map(str::to_owned).collect())
            .collect())
    }

    fn run(&self, sql: &str) -> Result<std::process::Output, StorageError> {
        let mut child = Command::new("sqlite3")
            .arg("-batch")
            .arg("-noheader")
            .arg("-separator")
            .arg(FIELD_SEPARATOR)
            .arg(&self.db_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut stdin = child.stdin.take().ok_or_else(|| StorageError::Sqlite {
            message: "failed to open sqlite stdin".to_owned(),
        })?;
        stdin.write_all(b"PRAGMA foreign_keys = ON;\n")?;
        stdin.write_all(sql.as_bytes())?;
        drop(stdin);
        child.wait_with_output().map_err(StorageError::from)
    }
}

pub(crate) fn sql_text(value: &str) -> Result<String, StorageError> {
    validate_text("sql_text", value, 1_000)?;
    Ok(format!("'{}'", value.replace('\'', "''")))
}

pub(crate) fn row_value<'a>(
    row: &'a [String],
    index: usize,
    field: &'static str,
) -> Result<&'a str, StorageError> {
    row.get(index)
        .map(String::as_str)
        .ok_or_else(|| StorageError::Sqlite {
            message: format!("missing column {field}"),
        })
}

fn sqlite_error(stderr: &[u8]) -> StorageError {
    StorageError::Sqlite {
        message: String::from_utf8_lossy(stderr).trim().to_owned(),
    }
}
