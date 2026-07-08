use std::{collections::BTreeSet, fs, path::Path};

use crate::sqlite_cli::{sql_text, Sqlite};
use crate::store::Store;
use crate::StorageError;

pub(crate) struct Migration {
    pub(crate) version: i64,
    pub(crate) name: &'static str,
    pub(crate) sql: &'static str,
}

pub(crate) const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "init",
        sql: include_str!("../migrations/0001_init.sql"),
    },
    Migration {
        version: 2,
        name: "feedback_eval",
        sql: include_str!("../migrations/0002_feedback_eval.sql"),
    },
    Migration {
        version: 3,
        name: "provider_route_outcomes",
        sql: include_str!("../migrations/0003_provider_route_outcomes.sql"),
    },
    Migration {
        version: 4,
        name: "sync_scheduler_state",
        sql: include_str!("../migrations/0004_sync_scheduler_state.sql"),
    },
    Migration {
        version: 5,
        name: "sync_scheduler_custom_interval",
        sql: include_str!("../migrations/0005_sync_scheduler_custom_interval.sql"),
    },
    Migration {
        version: 6,
        name: "quiet_log_provider_diagnostic",
        sql: include_str!("../migrations/0006_quiet_log_provider_diagnostic.sql"),
    },
    Migration {
        version: 7,
        name: "provider_route_profile_metadata",
        sql: include_str!("../migrations/0007_provider_route_profile_metadata.sql"),
    },
    Migration {
        version: 8,
        name: "list_intake_entries",
        sql: include_str!("../migrations/0008_list_intake_entries.sql"),
    },
    Migration {
        version: 9,
        name: "list_intake_provider_diagnostics",
        sql: include_str!("../migrations/0009_list_intake_provider_diagnostics.sql"),
    },
];

pub(crate) fn record_migration_sql(migration: &Migration) -> Result<String, StorageError> {
    Ok(format!(
        "INSERT OR IGNORE INTO _morrow_migrations (version, name, applied_at)
         VALUES ({version}, {name}, strftime('%s', 'now'));",
        version = migration.version,
        name = sql_text(migration.name)?,
    ))
}

impl Store {
    pub fn open(db_path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = db_path.parent().filter(|path| !path.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        let store = Self {
            sqlite: Sqlite::new(db_path),
            db_path: db_path.to_path_buf(),
        };
        let applied_versions = applied_migration_versions(&store.sqlite)?;
        let mut script = String::new();
        for migration in MIGRATIONS
            .iter()
            .filter(|migration| !applied_versions.contains(&migration.version))
        {
            script.push_str(migration.sql);
            script.push('\n');
            script.push_str(&record_migration_sql(migration)?);
            script.push('\n');
        }
        if !script.is_empty() {
            store.sqlite.execute(&script)?;
        }
        Ok(store)
    }
}

fn applied_migration_versions(sqlite: &Sqlite) -> Result<BTreeSet<i64>, StorageError> {
    sqlite
        .query_first_column(
            "CREATE TABLE IF NOT EXISTS _morrow_migrations (
                 version INTEGER PRIMARY KEY,
                 name TEXT NOT NULL UNIQUE,
                 applied_at INTEGER NOT NULL
             );
             SELECT version FROM _morrow_migrations ORDER BY version;",
        )?
        .into_iter()
        .map(|version| {
            version.parse::<i64>().map_err(|err| StorageError::Sqlite {
                message: format!("expected migration version integer: {err}"),
            })
        })
        .collect()
}
