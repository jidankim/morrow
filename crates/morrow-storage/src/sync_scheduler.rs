use crate::StorageError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncSchedulerIntervalSeconds(i64);

impl SyncSchedulerIntervalSeconds {
    pub const FIFTEEN_MINUTES: Self = Self(900);
    pub const THIRTY_MINUTES: Self = Self(1_800);
    pub const SIXTY_MINUTES: Self = Self(3_600);

    pub const fn as_i64(self) -> i64 {
        self.0
    }

    pub fn parse(raw: i64) -> Result<Self, StorageError> {
        match raw {
            900 => Ok(Self::FIFTEEN_MINUTES),
            1_800 => Ok(Self::THIRTY_MINUTES),
            3_600 => Ok(Self::SIXTY_MINUTES),
            other => Err(StorageError::InvalidInput {
                field: "interval_seconds",
                reason: format!("unsupported interval {other}"),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncSchedulerStatus {
    Disabled,
    Scheduled,
    Running,
    Cooldown,
    Blocked,
}

impl SyncSchedulerStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Scheduled => "scheduled",
            Self::Running => "running",
            Self::Cooldown => "cooldown",
            Self::Blocked => "blocked",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, StorageError> {
        match raw {
            "disabled" => Ok(Self::Disabled),
            "scheduled" => Ok(Self::Scheduled),
            "running" => Ok(Self::Running),
            "cooldown" => Ok(Self::Cooldown),
            "blocked" => Ok(Self::Blocked),
            other => Err(StorageError::InvalidInput {
                field: "status",
                reason: format!("unknown scheduler status {other}"),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncSchedulerLastResult {
    Success,
    RetryableFailure,
    Blocked,
    ManualDisabled,
}

impl SyncSchedulerLastResult {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::RetryableFailure => "retryable_failure",
            Self::Blocked => "blocked",
            Self::ManualDisabled => "manual_disabled",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, StorageError> {
        match raw {
            "success" => Ok(Self::Success),
            "retryable_failure" => Ok(Self::RetryableFailure),
            "blocked" => Ok(Self::Blocked),
            "manual_disabled" => Ok(Self::ManualDisabled),
            other => Err(StorageError::InvalidInput {
                field: "last_result",
                reason: format!("unknown scheduler result {other}"),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncSchedulerState {
    pub enabled: bool,
    pub interval_seconds: SyncSchedulerIntervalSeconds,
    pub status: SyncSchedulerStatus,
    pub last_started_at: Option<i64>,
    pub last_finished_at: Option<i64>,
    pub next_run_at: Option<i64>,
    pub next_eligible_at: Option<i64>,
    pub last_result: Option<SyncSchedulerLastResult>,
    pub retry_attempt: i64,
    pub last_reason: Option<String>,
    pub updated_at: i64,
}
