use crate::RemindersError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReminderDate {
    year: i32,
    month: u8,
    day: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReminderTime {
    hour: u8,
    minute: u8,
}

impl ReminderDate {
    pub fn parse(raw: &str) -> Result<Self, RemindersError> {
        let (year_raw, rest) = raw
            .split_once('-')
            .ok_or_else(|| invalid("due_date", "expected YYYY-MM-DD"))?;
        let (month_raw, day_raw) = rest
            .split_once('-')
            .ok_or_else(|| invalid("due_date", "expected YYYY-MM-DD"))?;
        if day_raw.contains('-') {
            return Err(invalid("due_date", "expected YYYY-MM-DD"));
        }
        let year = parse_i32(year_raw, "due_date")?;
        let month = parse_u8(month_raw, "due_date")?;
        let day = parse_u8(day_raw, "due_date")?;
        let max_day = days_in_month(year, month)?;
        if day == 0 || day > max_day {
            return Err(invalid("due_date", "day is out of range"));
        }
        Ok(Self { year, month, day })
    }

    pub fn as_str(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl ReminderTime {
    pub fn parse(raw: &str) -> Result<Self, RemindersError> {
        let (hour_raw, minute_raw) = raw
            .split_once(':')
            .ok_or_else(|| invalid("due_time", "expected HH:MM"))?;
        if minute_raw.contains(':') {
            return Err(invalid("due_time", "expected HH:MM"));
        }
        let hour = parse_u8(hour_raw, "due_time")?;
        let minute = parse_u8(minute_raw, "due_time")?;
        if hour > 23 {
            return Err(invalid("due_time", "hour is out of range"));
        }
        if minute > 59 {
            return Err(invalid("due_time", "minute is out of range"));
        }
        Ok(Self { hour, minute })
    }

    pub fn as_str(&self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }
}

fn parse_i32(raw: &str, field: &'static str) -> Result<i32, RemindersError> {
    if raw.len() != 4 || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid(field, "expected numeric component"));
    }
    raw.parse::<i32>()
        .map_err(|err| invalid(field, &format!("invalid number: {err}")))
}

fn parse_u8(raw: &str, field: &'static str) -> Result<u8, RemindersError> {
    if raw.len() != 2 || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid(field, "expected numeric component"));
    }
    raw.parse::<u8>()
        .map_err(|err| invalid(field, &format!("invalid number: {err}")))
}

fn days_in_month(year: i32, month: u8) -> Result<u8, RemindersError> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Ok(31),
        4 | 6 | 9 | 11 => Ok(30),
        2 if is_leap_year(year) => Ok(29),
        2 => Ok(28),
        _ => Err(invalid("due_date", "month is out of range")),
    }
}

const fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn invalid(field: &'static str, reason: &str) -> RemindersError {
    RemindersError::InvalidInput {
        field,
        reason: reason.to_owned(),
    }
}
