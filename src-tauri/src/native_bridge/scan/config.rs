use std::time::{SystemTime, UNIX_EPOCH};

use morrow_detection::{
    ConfidenceThreshold, DetectionConfig, DetectionError, ProviderIdentity, ReferenceTime,
    SourceExcerptPolicy,
};
use time::{Date, Month, OffsetDateTime, UtcOffset};

use super::{ScanSelectedChatsError, ScanSelectedChatsRequest};

const SUPPORTED_REFERENCE_TIMEZONES: &[&str] =
    &["Asia/Seoul", "America/New_York", "Europe/London", "UTC"];

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
    if !SUPPORTED_REFERENCE_TIMEZONES
        .iter()
        .any(|timezone| *timezone == request.reference_timezone)
    {
        return Err(ScanSelectedChatsError::Detection(
            "unsupported reference timezone".to_owned(),
        ));
    }
    let reference_time =
        reference_time_string(reference_unix_seconds, &request.reference_timezone)?;
    let source_excerpts = source_excerpt_policy(request.source_excerpts_enabled);
    Ok(ScanConfig {
        detection: DetectionConfig {
            reference: ReferenceTime::parse(&reference_time, &request.reference_timezone)
                .map_err(detection_error)?,
            threshold: ConfidenceThreshold::new(550).map_err(detection_error)?,
            provider: ProviderIdentity::new("native-bridge", "deterministic", "scan-v1")
                .map_err(detection_error)?,
            source_excerpts,
        },
        feedback_text_snapshots_enabled: request.source_excerpts_enabled
            && request.feedback_text_snapshots_enabled,
    })
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
    let local = utc.to_offset(timezone_offset(utc, timezone)?);
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

fn timezone_offset(
    utc: OffsetDateTime,
    timezone: &str,
) -> Result<UtcOffset, ScanSelectedChatsError> {
    match timezone {
        "UTC" => offset_hours(0),
        "Asia/Seoul" => offset_hours(9),
        "America/New_York" => {
            if new_york_daylight_time_utc(utc)? {
                offset_hours(-4)
            } else {
                offset_hours(-5)
            }
        }
        "Europe/London" => {
            if london_summer_time_utc(utc)? {
                offset_hours(1)
            } else {
                offset_hours(0)
            }
        }
        _other => Err(ScanSelectedChatsError::Detection(
            "unsupported reference timezone".to_owned(),
        )),
    }
}

fn offset_hours(hours: i8) -> Result<UtcOffset, ScanSelectedChatsError> {
    UtcOffset::from_hms(hours, 0, 0).map_err(time_error)
}

fn new_york_daylight_time_utc(utc: OffsetDateTime) -> Result<bool, ScanSelectedChatsError> {
    let year = utc.year();
    let starts = utc_boundary(year, Month::March, nth_sunday(year, Month::March, 2)?, 7)?;
    let ends = utc_boundary(
        year,
        Month::November,
        nth_sunday(year, Month::November, 1)?,
        6,
    )?;
    Ok(utc >= starts && utc < ends)
}

fn london_summer_time_utc(utc: OffsetDateTime) -> Result<bool, ScanSelectedChatsError> {
    let year = utc.year();
    let starts = utc_boundary(year, Month::March, last_sunday(year, Month::March)?, 1)?;
    let ends = utc_boundary(year, Month::October, last_sunday(year, Month::October)?, 1)?;
    Ok(utc >= starts && utc < ends)
}

fn utc_boundary(
    year: i32,
    month: Month,
    day: u8,
    hour: u8,
) -> Result<OffsetDateTime, ScanSelectedChatsError> {
    Date::from_calendar_date(year, month, day)
        .and_then(|date| date.with_hms(hour, 0, 0))
        .map(PrimitiveDateTimeExt::assume_utc_datetime)
        .map_err(time_error)
}

trait PrimitiveDateTimeExt {
    fn assume_utc_datetime(self) -> OffsetDateTime;
}

impl PrimitiveDateTimeExt for time::PrimitiveDateTime {
    fn assume_utc_datetime(self) -> OffsetDateTime {
        self.assume_utc()
    }
}

fn nth_sunday(year: i32, month: Month, ordinal: u8) -> Result<u8, ScanSelectedChatsError> {
    let first = Date::from_calendar_date(year, month, 1).map_err(time_error)?;
    let offset = (7 - first.weekday().number_days_from_sunday()) % 7;
    Ok(1 + offset + 7 * (ordinal - 1))
}

fn last_sunday(year: i32, month: Month) -> Result<u8, ScanSelectedChatsError> {
    let last_day = month.length(year);
    let last = Date::from_calendar_date(year, month, last_day).map_err(time_error)?;
    Ok(last_day - last.weekday().number_days_from_sunday())
}

fn detection_error(error: DetectionError) -> ScanSelectedChatsError {
    ScanSelectedChatsError::Detection(error.to_string())
}

fn time_error(error: time::error::ComponentRange) -> ScanSelectedChatsError {
    ScanSelectedChatsError::Detection(format!("invalid reference timestamp: {error}"))
}
