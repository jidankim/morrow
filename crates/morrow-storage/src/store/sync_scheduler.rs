use crate::sqlite_cli::{row_value, SqliteValue};
use crate::sync_scheduler::{
    SyncSchedulerIntervalSeconds, SyncSchedulerLastResult, SyncSchedulerState, SyncSchedulerStatus,
};
use crate::validation::validate_text;
use crate::StorageError;

use super::Store;

impl Store {
    pub fn load_sync_scheduler_state(&self) -> Result<SyncSchedulerState, StorageError> {
        let rows = self.sqlite.query_rows(
            "SELECT enabled, interval_seconds, status, last_started_at, last_finished_at,
                    next_run_at, next_eligible_at, last_result, retry_attempt, last_reason,
                    updated_at
             FROM sync_scheduler_state
             WHERE id = 1;",
        )?;
        let row = rows.first().ok_or_else(|| StorageError::Sqlite {
            message: "sync scheduler singleton row missing".to_owned(),
        })?;
        parse_sync_scheduler_row(row)
    }

    pub fn save_sync_scheduler_state(
        &self,
        state: &SyncSchedulerState,
    ) -> Result<(), StorageError> {
        validate_sync_scheduler_state(state)?;
        self.sqlite.execute_with_params(
            "INSERT INTO sync_scheduler_state
             (id, enabled, interval_seconds, status, last_started_at, last_finished_at,
              next_run_at, next_eligible_at, last_result, retry_attempt, last_reason, updated_at)
             VALUES
             (1, :p0, :p1, :p2, :p3, :p4, :p5, :p6, :p7, :p8, :p9, :p10)
             ON CONFLICT(id) DO UPDATE SET
                enabled = excluded.enabled,
                interval_seconds = excluded.interval_seconds,
                status = excluded.status,
                last_started_at = excluded.last_started_at,
                last_finished_at = excluded.last_finished_at,
                next_run_at = excluded.next_run_at,
                next_eligible_at = excluded.next_eligible_at,
                last_result = excluded.last_result,
                retry_attempt = excluded.retry_attempt,
                last_reason = excluded.last_reason,
                updated_at = excluded.updated_at;",
            &[
                SqliteValue::Integer(bool_int(state.enabled)),
                SqliteValue::Integer(state.interval_seconds.as_i64()),
                SqliteValue::Text(state.status.as_str()),
                optional_i64(state.last_started_at),
                optional_i64(state.last_finished_at),
                optional_i64(state.next_run_at),
                optional_i64(state.next_eligible_at),
                optional_result(state.last_result),
                SqliteValue::Integer(state.retry_attempt),
                optional_text(state.last_reason.as_deref()),
                SqliteValue::Integer(state.updated_at),
            ],
        )
    }
}

fn parse_sync_scheduler_row(row: &[String]) -> Result<SyncSchedulerState, StorageError> {
    Ok(SyncSchedulerState {
        enabled: parse_bool(row_value(row, 0, "enabled")?)?,
        interval_seconds: SyncSchedulerIntervalSeconds::parse(parse_i64(
            row_value(row, 1, "interval_seconds")?,
            "interval_seconds",
        )?)?,
        status: SyncSchedulerStatus::parse(row_value(row, 2, "status")?)?,
        last_started_at: parse_optional_i64(row_value(row, 3, "last_started_at")?)?,
        last_finished_at: parse_optional_i64(row_value(row, 4, "last_finished_at")?)?,
        next_run_at: parse_optional_i64(row_value(row, 5, "next_run_at")?)?,
        next_eligible_at: parse_optional_i64(row_value(row, 6, "next_eligible_at")?)?,
        last_result: parse_optional_result(row_value(row, 7, "last_result")?)?,
        retry_attempt: parse_i64(row_value(row, 8, "retry_attempt")?, "retry_attempt")?,
        last_reason: parse_optional_text(row_value(row, 9, "last_reason")?),
        updated_at: parse_i64(row_value(row, 10, "updated_at")?, "updated_at")?,
    })
}

fn validate_sync_scheduler_state(state: &SyncSchedulerState) -> Result<(), StorageError> {
    if state.retry_attempt < 0 {
        return Err(StorageError::InvalidInput {
            field: "retry_attempt",
            reason: "must be non-negative".to_owned(),
        });
    }
    if let Some(reason) = &state.last_reason {
        validate_text("last_reason", reason, 240)?;
    }
    Ok(())
}

fn parse_bool(raw: &str) -> Result<bool, StorageError> {
    match raw {
        "0" => Ok(false),
        "1" => Ok(true),
        other => Err(StorageError::InvalidInput {
            field: "enabled",
            reason: format!("expected 0 or 1, got {other}"),
        }),
    }
}

fn parse_optional_i64(raw: &str) -> Result<Option<i64>, StorageError> {
    if raw.is_empty() {
        Ok(None)
    } else {
        parse_i64(raw, "nullable timestamp").map(Some)
    }
}

fn parse_optional_result(raw: &str) -> Result<Option<SyncSchedulerLastResult>, StorageError> {
    if raw.is_empty() {
        Ok(None)
    } else {
        SyncSchedulerLastResult::parse(raw).map(Some)
    }
}

fn parse_optional_text(raw: &str) -> Option<String> {
    if raw.is_empty() {
        None
    } else {
        Some(raw.to_owned())
    }
}

fn parse_i64(raw: &str, field: &'static str) -> Result<i64, StorageError> {
    raw.parse::<i64>()
        .map_err(|err| StorageError::InvalidInput {
            field,
            reason: err.to_string(),
        })
}

fn bool_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn optional_i64(value: Option<i64>) -> SqliteValue<'static> {
    match value {
        Some(value) => SqliteValue::Integer(value),
        None => SqliteValue::Null,
    }
}

fn optional_result(value: Option<SyncSchedulerLastResult>) -> SqliteValue<'static> {
    match value {
        Some(value) => SqliteValue::Text(value.as_str()),
        None => SqliteValue::Null,
    }
}

fn optional_text(value: Option<&str>) -> SqliteValue<'_> {
    match value {
        Some(value) => SqliteValue::Text(value),
        None => SqliteValue::Null,
    }
}
