use std::{
    collections::HashMap,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{Mutex, OnceLock},
    thread::{self, ThreadId},
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
const ALL_CHAT_GUIDS_SQL: &str = "SELECT guid FROM chat ORDER BY guid ASC;";

static SQLITE_QUERY_AUDIT: OnceLock<Mutex<HashMap<ThreadId, SqliteQueryAudit>>> = OnceLock::new();

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SqliteQueryAudit {
    pub total_query_count: usize,
    pub all_chat_guids_sql_count: usize,
}

#[derive(Debug)]
pub struct SqliteQueryAuditGuard {
    owner: ThreadId,
}

fn sqlite_query_audit() -> &'static Mutex<HashMap<ThreadId, SqliteQueryAudit>> {
    SQLITE_QUERY_AUDIT.get_or_init(|| Mutex::new(HashMap::new()))
}

#[doc(hidden)]
pub fn begin_sqlite_query_audit() -> SqliteQueryAuditGuard {
    let owner = thread::current().id();
    if let Ok(mut audit) = sqlite_query_audit().lock() {
        audit.insert(owner, SqliteQueryAudit::default());
    }
    SqliteQueryAuditGuard { owner }
}

#[doc(hidden)]
pub fn sqlite_query_audit_snapshot() -> SqliteQueryAudit {
    let owner = thread::current().id();
    sqlite_query_audit()
        .lock()
        .ok()
        .and_then(|audit| audit.get(&owner).cloned())
        .unwrap_or_default()
}

impl Drop for SqliteQueryAuditGuard {
    fn drop(&mut self) {
        if let Ok(mut audit) = sqlite_query_audit().lock() {
            audit.remove(&self.owner);
        }
    }
}

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
        record_sqlite_query(sql);
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

fn record_sqlite_query(sql: &str) {
    let owner = thread::current().id();
    if let Ok(mut audit) = sqlite_query_audit().lock() {
        if let Some(snapshot) = audit.get_mut(&owner) {
            snapshot.total_query_count += 1;
            if sql.trim() == ALL_CHAT_GUIDS_SQL {
                snapshot.all_chat_guids_sql_count += 1;
            }
        }
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
    use std::{
        path::Path,
        process::Command,
        sync::{Arc, Barrier},
        thread,
    };

    use super::{
        begin_sqlite_query_audit, record_sqlite_query, sqlite_query_audit_snapshot, Sqlite,
        ALL_CHAT_GUIDS_SQL,
    };

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

    #[test]
    fn sqlite_query_audits_are_thread_isolated() -> Result<(), String> {
        // Given
        let _main_audit = begin_sqlite_query_audit();
        let child_started = Arc::new(Barrier::new(2));
        let queries_recorded = Arc::new(Barrier::new(2));
        let snapshots_captured = Arc::new(Barrier::new(2));
        let child_handle = {
            let child_started = Arc::clone(&child_started);
            let queries_recorded = Arc::clone(&queries_recorded);
            let snapshots_captured = Arc::clone(&snapshots_captured);
            thread::spawn(move || {
                let _child_audit = begin_sqlite_query_audit();
                child_started.wait();

                // When
                record_sqlite_query(ALL_CHAT_GUIDS_SQL);
                queries_recorded.wait();
                let child_snapshot = sqlite_query_audit_snapshot();
                snapshots_captured.wait();

                child_snapshot
            })
        };
        child_started.wait();

        // When
        record_sqlite_query("SELECT 1;");
        queries_recorded.wait();
        let main_snapshot = sqlite_query_audit_snapshot();
        snapshots_captured.wait();
        let child_snapshot = child_handle
            .join()
            .map_err(|_| "child audit thread panicked".to_owned())?;

        // Then
        assert_eq!(
            main_snapshot,
            super::SqliteQueryAudit {
                total_query_count: 1,
                all_chat_guids_sql_count: 0,
            }
        );
        assert_eq!(
            child_snapshot,
            super::SqliteQueryAudit {
                total_query_count: 1,
                all_chat_guids_sql_count: 1,
            }
        );
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
