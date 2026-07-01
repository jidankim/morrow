use std::path::Path;

use morrow_storage::{
    Store, SyncSchedulerIntervalSeconds, SyncSchedulerLastResult, SyncSchedulerState,
    SyncSchedulerStatus,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncSchedulerStatusCommand {
    Disabled,
    Scheduled,
    Running,
    Cooldown,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncSchedulerLastResultCommand {
    Success,
    RetryableFailure,
    Blocked,
    ManualDisabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncSchedulerStateCommand {
    pub enabled: bool,
    pub interval_seconds: i64,
    pub status: SyncSchedulerStatusCommand,
    pub last_started_at: Option<i64>,
    pub last_finished_at: Option<i64>,
    pub next_run_at: Option<i64>,
    pub next_eligible_at: Option<i64>,
    pub last_result: Option<SyncSchedulerLastResultCommand>,
    pub retry_attempt: i64,
    pub last_reason: Option<String>,
    pub updated_at: i64,
}

pub fn load_sync_scheduler_state_at(
    store_path: &Path,
) -> Result<SyncSchedulerStateCommand, String> {
    let store = Store::open(store_path).map_err(|error| error.to_string())?;
    store
        .load_sync_scheduler_state()
        .map(SyncSchedulerStateCommand::from)
        .map_err(|error| error.to_string())
}

pub fn save_sync_scheduler_state_at(
    store_path: &Path,
    state: SyncSchedulerStateCommand,
) -> Result<SyncSchedulerStateCommand, String> {
    let store = Store::open(store_path).map_err(|error| error.to_string())?;
    let storage_state = SyncSchedulerState::try_from(state.clone())?;
    store
        .save_sync_scheduler_state(&storage_state)
        .map_err(|error| error.to_string())?;
    Ok(state)
}

impl TryFrom<SyncSchedulerStateCommand> for SyncSchedulerState {
    type Error = String;

    fn try_from(value: SyncSchedulerStateCommand) -> Result<Self, Self::Error> {
        validate_retry_attempt(value.retry_attempt)?;
        Ok(Self {
            enabled: value.enabled,
            interval_seconds: SyncSchedulerIntervalSeconds::parse(value.interval_seconds)
                .map_err(|error| error.to_string())?,
            status: SyncSchedulerStatus::from(value.status),
            last_started_at: value.last_started_at,
            last_finished_at: value.last_finished_at,
            next_run_at: value.next_run_at,
            next_eligible_at: value.next_eligible_at,
            last_result: value.last_result.map(SyncSchedulerLastResult::from),
            retry_attempt: value.retry_attempt,
            last_reason: value.last_reason,
            updated_at: value.updated_at,
        })
    }
}

impl From<SyncSchedulerState> for SyncSchedulerStateCommand {
    fn from(value: SyncSchedulerState) -> Self {
        Self {
            enabled: value.enabled,
            interval_seconds: value.interval_seconds.as_i64(),
            status: SyncSchedulerStatusCommand::from(value.status),
            last_started_at: value.last_started_at,
            last_finished_at: value.last_finished_at,
            next_run_at: value.next_run_at,
            next_eligible_at: value.next_eligible_at,
            last_result: value.last_result.map(SyncSchedulerLastResultCommand::from),
            retry_attempt: value.retry_attempt,
            last_reason: value.last_reason,
            updated_at: value.updated_at,
        }
    }
}

impl From<SyncSchedulerStatusCommand> for SyncSchedulerStatus {
    fn from(value: SyncSchedulerStatusCommand) -> Self {
        match value {
            SyncSchedulerStatusCommand::Disabled => Self::Disabled,
            SyncSchedulerStatusCommand::Scheduled => Self::Scheduled,
            SyncSchedulerStatusCommand::Running => Self::Running,
            SyncSchedulerStatusCommand::Cooldown => Self::Cooldown,
            SyncSchedulerStatusCommand::Blocked => Self::Blocked,
        }
    }
}

impl From<SyncSchedulerStatus> for SyncSchedulerStatusCommand {
    fn from(value: SyncSchedulerStatus) -> Self {
        match value {
            SyncSchedulerStatus::Disabled => Self::Disabled,
            SyncSchedulerStatus::Scheduled => Self::Scheduled,
            SyncSchedulerStatus::Running => Self::Running,
            SyncSchedulerStatus::Cooldown => Self::Cooldown,
            SyncSchedulerStatus::Blocked => Self::Blocked,
        }
    }
}

impl From<SyncSchedulerLastResultCommand> for SyncSchedulerLastResult {
    fn from(value: SyncSchedulerLastResultCommand) -> Self {
        match value {
            SyncSchedulerLastResultCommand::Success => Self::Success,
            SyncSchedulerLastResultCommand::RetryableFailure => Self::RetryableFailure,
            SyncSchedulerLastResultCommand::Blocked => Self::Blocked,
            SyncSchedulerLastResultCommand::ManualDisabled => Self::ManualDisabled,
        }
    }
}

impl From<SyncSchedulerLastResult> for SyncSchedulerLastResultCommand {
    fn from(value: SyncSchedulerLastResult) -> Self {
        match value {
            SyncSchedulerLastResult::Success => Self::Success,
            SyncSchedulerLastResult::RetryableFailure => Self::RetryableFailure,
            SyncSchedulerLastResult::Blocked => Self::Blocked,
            SyncSchedulerLastResult::ManualDisabled => Self::ManualDisabled,
        }
    }
}

fn validate_retry_attempt(retry_attempt: i64) -> Result<(), String> {
    if retry_attempt >= 0 {
        return Ok(());
    }
    Err(format!("invalid retry_attempt: {retry_attempt}"))
}
