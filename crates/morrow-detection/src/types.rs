use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectionConfig {
    pub reference: ReferenceTime,
    pub threshold: ConfidenceThreshold,
    pub provider: ProviderIdentity,
    pub source_excerpts: SourceExcerptPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceExcerptPolicy {
    Include,
    Hide,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderIdentity {
    pub provider_id: String,
    pub model_id: String,
    pub prompt_version: String,
}

impl ProviderIdentity {
    pub fn new(
        provider_id: &str,
        model_id: &str,
        prompt_version: &str,
    ) -> Result<Self, DetectionError> {
        Ok(Self {
            provider_id: bounded("provider_id", provider_id, 80)?,
            model_id: bounded("model_id", model_id, 120)?,
            prompt_version: bounded("prompt_version", prompt_version, 80)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceTime {
    pub observed: CivilDateTime,
    pub timezone: String,
}

impl ReferenceTime {
    pub fn parse(raw: &str, timezone: &str) -> Result<Self, DetectionError> {
        Ok(Self {
            observed: CivilDateTime::parse_reference(raw)?,
            timezone: bounded("timezone", timezone, 80)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConfidenceThreshold(i64);

impl ConfidenceThreshold {
    pub const fn new(value: i64) -> Result<Self, DetectionError> {
        if value >= 0 && value <= 1000 {
            Ok(Self(value))
        } else {
            Err(DetectionError::InvalidInput {
                field: "threshold_millis",
                reason: "must be between 0 and 1000",
            })
        }
    }

    pub const fn as_i64(self) -> i64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CivilDateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
}

impl CivilDateTime {
    pub fn parse_reference(raw: &str) -> Result<Self, DetectionError> {
        let (date, time) = raw.split_once('T').ok_or(DetectionError::InvalidInput {
            field: "reference_time",
            reason: "must use YYYY-MM-DDTHH:MM:SS",
        })?;
        let (hour, minute) = parse_time(time)?;
        parse_date_time(date, hour, minute, "reference_time")
    }

    pub fn parse_normalized(raw: &str) -> Result<Self, DetectionError> {
        let (date, rest) = raw.split_once('T').ok_or(DetectionError::InvalidInput {
            field: "normalized_time",
            reason: "must use YYYY-MM-DDTHH:MM:SS[Timezone]",
        })?;
        let time = rest.split_once('[').map_or(rest, |parts| parts.0);
        let (hour, minute) = parse_time(time)?;
        parse_date_time(date, hour, minute, "normalized_time")
    }

    pub fn normalized(self, timezone: &str) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:00[{}]",
            self.year, self.month, self.day, self.hour, self.minute, timezone
        )
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DetectionError {
    #[error("invalid {field}: {reason}")]
    InvalidInput {
        field: &'static str,
        reason: &'static str,
    },
}

impl Display for ConfidenceThreshold {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn bounded(field: &'static str, value: &str, max_len: usize) -> Result<String, DetectionError> {
    if value.is_empty() {
        return Err(DetectionError::InvalidInput {
            field,
            reason: "must not be empty",
        });
    }
    if value.len() > max_len {
        return Err(DetectionError::InvalidInput {
            field,
            reason: "is too long",
        });
    }
    Ok(value.to_owned())
}

fn parse_date_time(
    date: &str,
    hour: u8,
    minute: u8,
    field: &'static str,
) -> Result<CivilDateTime, DetectionError> {
    let mut parts = date.split('-');
    let year = parse_part::<u16>(&mut parts, field)?;
    let month = parse_part::<u8>(&mut parts, field)?;
    let day = parse_part::<u8>(&mut parts, field)?;
    let Some(max_day) = days_in_month(year, month) else {
        return Err(DetectionError::InvalidInput {
            field,
            reason: "contains an invalid date",
        });
    };
    if parts.next().is_some() || day == 0 || day > max_day {
        return Err(DetectionError::InvalidInput {
            field,
            reason: "contains an invalid date",
        });
    }
    Ok(CivilDateTime {
        year,
        month,
        day,
        hour,
        minute,
    })
}

fn parse_time(time: &str) -> Result<(u8, u8), DetectionError> {
    let without_seconds = time.split_once(':').ok_or(DetectionError::InvalidInput {
        field: "time",
        reason: "must include hour and minute",
    })?;
    let hour = without_seconds
        .0
        .parse::<u8>()
        .map_err(|_| DetectionError::InvalidInput {
            field: "time",
            reason: "contains an invalid hour",
        })?;
    let minute_text = without_seconds
        .1
        .split_once(':')
        .map_or(without_seconds.1, |parts| parts.0);
    let minute = minute_text
        .parse::<u8>()
        .map_err(|_| DetectionError::InvalidInput {
            field: "time",
            reason: "contains an invalid minute",
        })?;
    if hour > 23 || minute > 59 {
        return Err(DetectionError::InvalidInput {
            field: "time",
            reason: "contains an invalid clock value",
        });
    }
    Ok((hour, minute))
}

fn parse_part<T: std::str::FromStr>(
    parts: &mut std::str::Split<'_, char>,
    field: &'static str,
) -> Result<T, DetectionError> {
    parts
        .next()
        .ok_or(DetectionError::InvalidInput {
            field,
            reason: "is incomplete",
        })?
        .parse::<T>()
        .map_err(|_| DetectionError::InvalidInput {
            field,
            reason: "contains non-numeric date parts",
        })
}

const fn days_in_month(year: u16, month: u8) -> Option<u8> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 if is_leap_year(year) => Some(29),
        2 => Some(28),
        _ => None,
    }
}

const fn is_leap_year(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}
