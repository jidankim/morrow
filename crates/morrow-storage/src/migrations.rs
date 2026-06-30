use crate::sqlite_cli::sql_text;
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
];

pub(crate) fn record_migration_sql(migration: &Migration) -> Result<String, StorageError> {
    Ok(format!(
        "INSERT OR IGNORE INTO _morrow_migrations (version, name, applied_at)
         VALUES ({version}, {name}, strftime('%s', 'now'));",
        version = migration.version,
        name = sql_text(migration.name)?,
    ))
}
