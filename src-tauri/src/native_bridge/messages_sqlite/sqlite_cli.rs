use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use morrow_messages::MessagesError;

const FIELD_SEPARATOR: &str = "\u{1f}";
const SQLITE_ARGS: &[&str] = &[
    "-batch",
    "-noheader",
    "-readonly",
    "-separator",
    FIELD_SEPARATOR,
];
const STARTUP_SQL: &str = "PRAGMA query_only = ON;\nPRAGMA trusted_schema = OFF;\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SqliteReadProtections {
    pub sqlite3_args: &'static [&'static str],
    pub startup_sql: &'static str,
}

#[derive(Debug, Clone)]
pub struct Sqlite {
    db_path: PathBuf,
}

impl Sqlite {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    pub const fn protections() -> SqliteReadProtections {
        SqliteReadProtections {
            sqlite3_args: SQLITE_ARGS,
            startup_sql: STARTUP_SQL,
        }
    }

    pub fn query_rows(&self, sql: &str) -> Result<Vec<Vec<String>>, MessagesError> {
        let output = self.run(sql)?;
        if !output.status.success() {
            return Err(sqlite_error_from_stderr(&output.stderr));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| line.split(FIELD_SEPARATOR).map(str::to_owned).collect())
            .collect())
    }

    fn run(&self, sql: &str) -> Result<std::process::Output, MessagesError> {
        let mut child = Command::new("sqlite3")
            .args(SQLITE_ARGS)
            .arg(&self.db_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| unavailable(format!("failed to start sqlite3: {error}")))?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| unavailable("failed to open sqlite stdin"))?;
        stdin
            .write_all(STARTUP_SQL.as_bytes())
            .map_err(|error| unavailable(format!("failed to configure sqlite3: {error}")))?;
        stdin
            .write_all(sql.as_bytes())
            .map_err(|error| unavailable(format!("failed to write sqlite query: {error}")))?;
        drop(stdin);
        child
            .wait_with_output()
            .map_err(|error| unavailable(format!("sqlite3 did not finish: {error}")))
    }
}

pub(crate) fn sqlite_error_from_stderr(stderr: &[u8]) -> MessagesError {
    let message = String::from_utf8_lossy(stderr).to_ascii_lowercase();
    if message.contains("permission denied")
        || message.contains("operation not permitted")
        || message.contains("authorization denied")
        || message.contains("not authorized")
    {
        MessagesError::PermissionDenied
    } else {
        unavailable("sqlite3 could not read the Messages database")
    }
}

fn unavailable(reason: impl Into<String>) -> MessagesError {
    MessagesError::NativeUnavailable {
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use std::{path::Path, process::Command};

    use super::Sqlite;

    #[test]
    fn protected_query_blocks_write_attempts() -> Result<(), String> {
        // Given
        let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let db_path = dir.path().join("protected-chat.db");
        run_sqlite(
            &db_path,
            "CREATE TABLE write_probe (value TEXT); INSERT INTO write_probe (value) VALUES ('before');",
        )?;
        let sqlite = Sqlite::new(db_path.clone());

        // When
        let write_result = sqlite.query_rows("INSERT INTO write_probe (value) VALUES ('after');");
        let rows = query_sqlite(&db_path, "SELECT COUNT(*) FROM write_probe;")?;

        // Then
        assert!(write_result.is_err());
        assert_eq!(rows, vec![vec!["1".to_owned()]]);
        Ok(())
    }

    fn run_sqlite(db_path: &Path, sql: &str) -> Result<(), String> {
        let output = Command::new("sqlite3")
            .arg(db_path)
            .arg(sql)
            .output()
            .map_err(|error| error.to_string())?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
        }
    }

    fn query_sqlite(db_path: &Path, sql: &str) -> Result<Vec<Vec<String>>, String> {
        let output = Command::new("sqlite3")
            .arg("-batch")
            .arg("-noheader")
            .arg("-separator")
            .arg("\u{1f}")
            .arg(db_path)
            .arg(sql)
            .output()
            .map_err(|error| error.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| line.split('\u{1f}').map(str::to_owned).collect())
            .collect())
    }
}
