use std::time::{SystemTime, UNIX_EPOCH};

use morrow_detection::{
    ConfidenceThreshold, DetectionConfig, DetectionError, ProviderIdentity, ReferenceTime,
    SourceExcerptPolicy,
};
use time::OffsetDateTime;
use time_tz::{timezones, OffsetDateTimeExt};

use super::{ScanSelectedChatsError, ScanSelectedChatsRequest};

const MIN_LOCAL_DIAGNOSTICS_RETENTION_DAYS: u16 = 1;
const MAX_LOCAL_DIAGNOSTICS_RETENTION_DAYS: u16 = 365;

pub(super) struct ScanConfig {
    pub detection: DetectionConfig,
    pub feedback_text_snapshots_enabled: bool,
}

pub(super) fn reference_unix_seconds(
    request: &ScanSelectedChatsRequest,
) -> Result<i64, ScanSelectedChatsError> {
    match request.reference_unix_seconds {
        Some(seconds) => Ok(seconds),
        None => current_unix_seconds(),
    }
}

pub(super) fn scan_config(
    request: &ScanSelectedChatsRequest,
    reference_unix_seconds: i64,
) -> Result<ScanConfig, ScanSelectedChatsError> {
    let reference_time =
        reference_time_string(reference_unix_seconds, &request.reference_timezone)?;
    let source_excerpts = source_excerpt_policy(request.source_excerpts_enabled);
    Ok(ScanConfig {
        detection: DetectionConfig {
            reference: ReferenceTime::parse(&reference_time, &request.reference_timezone)
                .map_err(detection_error)?,
            threshold: ConfidenceThreshold::new(550).map_err(detection_error)?,
            provider: ProviderIdentity::new("native-bridge", "deterministic", "scan-v2")
                .map_err(detection_error)?,
            source_excerpts,
        },
        feedback_text_snapshots_enabled: request.source_excerpts_enabled
            && request.feedback_text_snapshots_enabled,
    })
}

pub(super) fn validate_local_diagnostics_retention(
    request: &ScanSelectedChatsRequest,
) -> Result<(), ScanSelectedChatsError> {
    if (MIN_LOCAL_DIAGNOSTICS_RETENTION_DAYS..=MAX_LOCAL_DIAGNOSTICS_RETENTION_DAYS)
        .contains(&request.local_diagnostics_retention_days)
    {
        Ok(())
    } else {
        Err(ScanSelectedChatsError::Detection(
            "local diagnostics retention days must be between 1 and 365".to_owned(),
        ))
    }
}

const fn source_excerpt_policy(enabled: bool) -> SourceExcerptPolicy {
    if enabled {
        SourceExcerptPolicy::Include
    } else {
        SourceExcerptPolicy::Hide
    }
}

fn current_unix_seconds() -> Result<i64, ScanSelectedChatsError> {
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| {
        ScanSelectedChatsError::Detection("system clock is before UNIX epoch".to_owned())
    })?;
    i64::try_from(duration.as_secs()).map_err(|_| {
        ScanSelectedChatsError::Detection("system clock timestamp is too large".to_owned())
    })
}

fn reference_time_string(
    unix_seconds: i64,
    timezone: &str,
) -> Result<String, ScanSelectedChatsError> {
    let utc = OffsetDateTime::from_unix_timestamp(unix_seconds).map_err(time_error)?;
    let local = if timezone == "UTC" {
        utc
    } else {
        let timezone = timezones::get_by_name(timezone).ok_or_else(|| {
            ScanSelectedChatsError::Detection("unsupported reference timezone".to_owned())
        })?;
        utc.to_timezone(timezone)
    };
    Ok(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        local.year(),
        u8::from(local.month()),
        local.day(),
        local.hour(),
        local.minute(),
        local.second()
    ))
}

fn detection_error(error: DetectionError) -> ScanSelectedChatsError {
    ScanSelectedChatsError::Detection(error.to_string())
}

fn time_error(error: time::error::ComponentRange) -> ScanSelectedChatsError {
    ScanSelectedChatsError::Detection(format!("invalid reference timestamp: {error}"))
}
