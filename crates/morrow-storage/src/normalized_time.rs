use crate::StorageError;

const MAX_NORMALIZED_TIME_BYTES: usize = 64;
const SAFE_ZONE_AREAS: &[&str] = &[
    "Africa",
    "America",
    "Antarctica",
    "Arctic",
    "Asia",
    "Atlantic",
    "Australia",
    "Europe",
    "Indian",
    "Pacific",
    "Etc",
];
const FORBIDDEN_ZONE_STEMS: &[&str] = &[
    "private",
    "prompt",
    "response",
    "fullmessage",
    "rawmessage",
    "messagebody",
    "rawtext",
    "messagehistory",
];

pub fn validate_normalized_time(value: &str) -> Result<(), StorageError> {
    if value.len() > MAX_NORMALIZED_TIME_BYTES {
        return invalid("must be at most 64 bytes");
    }
    let (date, rest) = value
        .split_once('T')
        .ok_or_else(|| invalid_value("must use YYYY-MM-DDTHH:MM:SS[Timezone]"))?;
    let (time, zone) = split_time_zone(rest)?;
    validate_fixed_width_date(date)?;
    validate_fixed_width_time(time)?;
    let (hour, minute, second) = parse_time(time)?;
    parse_date_time(date, hour, minute, second)?;
    validate_zone(zone)
}

fn parse_date_time(date: &str, hour: u8, minute: u8, second: u8) -> Result<(), StorageError> {
    let mut parts = date.split('-');
    let year = parse_part::<u16>(&mut parts, "contains an invalid date")?;
    let month = parse_part::<u8>(&mut parts, "contains an invalid date")?;
    let day = parse_part::<u8>(&mut parts, "contains an invalid date")?;
    let Some(max_day) = days_in_month(year, month) else {
        return invalid("contains an invalid date");
    };
    if parts.next().is_some() || day == 0 || day > max_day {
        return invalid("contains an invalid date");
    }
    if hour > 23 || minute > 59 || second > 59 {
        return invalid("contains an invalid time");
    }
    Ok(())
}

fn split_time_zone(rest: &str) -> Result<(&str, &str), StorageError> {
    if let Some(time) = rest.strip_suffix('Z') {
        return Ok((time, "Z"));
    }
    let Some(start) = rest.find('[') else {
        return invalid("must end with Z or [Timezone]");
    };
    if !rest.ends_with(']') {
        return invalid("must end with Z or [Timezone]");
    }
    Ok((&rest[..start], &rest[start..]))
}

fn parse_time(time: &str) -> Result<(u8, u8, u8), StorageError> {
    let (hour_text, rest) = time
        .split_once(':')
        .ok_or_else(|| invalid_value("must include hour and minute"))?;
    let (minute_text, second_text) = rest
        .split_once(':')
        .ok_or_else(|| invalid_value("must include seconds"))?;
    if second_text.contains(':') {
        return invalid("contains an invalid time");
    }
    let hour = hour_text
        .parse::<u8>()
        .map_err(|_| invalid_value("contains an invalid hour"))?;
    let minute = minute_text
        .parse::<u8>()
        .map_err(|_| invalid_value("contains an invalid minute"))?;
    let second = second_text
        .parse::<u8>()
        .map_err(|_| invalid_value("contains an invalid second"))?;
    Ok((hour, minute, second))
}

fn validate_zone(zone: &str) -> Result<(), StorageError> {
    if zone == "Z" {
        return Ok(());
    }
    let Some(name) = zone
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    else {
        return invalid("must end with Z or [Timezone]");
    };
    if name.len() > 40 || !is_safe_iana_zone(name) {
        return invalid("contains an unsupported timezone");
    }
    Ok(())
}

fn validate_fixed_width_date(date: &str) -> Result<(), StorageError> {
    let bytes = date.as_bytes();
    if bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[8..].iter().all(u8::is_ascii_digit)
    {
        Ok(())
    } else {
        invalid("must use fixed-width date and time")
    }
}

fn validate_fixed_width_time(time: &str) -> Result<(), StorageError> {
    let bytes = time.as_bytes();
    if bytes.len() == 8
        && bytes[2] == b':'
        && bytes[5] == b':'
        && bytes[..2].iter().all(u8::is_ascii_digit)
        && bytes[3..5].iter().all(u8::is_ascii_digit)
        && bytes[6..].iter().all(u8::is_ascii_digit)
    {
        Ok(())
    } else {
        invalid("must use fixed-width date and time")
    }
}

fn is_safe_iana_zone(name: &str) -> bool {
    let mut parts = name.split('/');
    let Some(area) = parts.next() else {
        return false;
    };
    if !SAFE_ZONE_AREAS.contains(&area) {
        return false;
    }
    let mut has_location = false;
    for part in parts {
        if !is_safe_zone_component(part) {
            return false;
        }
        has_location = true;
    }
    has_location
}

fn is_safe_zone_component(part: &str) -> bool {
    let mut chars = part.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_uppercase()
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
        && !contains_forbidden_zone_term(part)
}

fn contains_forbidden_zone_term(part: &str) -> bool {
    let normalized: String = part
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect();
    FORBIDDEN_ZONE_STEMS
        .iter()
        .any(|forbidden| normalized.contains(forbidden))
}

fn parse_part<T: std::str::FromStr>(
    parts: &mut std::str::Split<'_, char>,
    reason: &'static str,
) -> Result<T, StorageError> {
    parts
        .next()
        .ok_or_else(|| invalid_value(reason))?
        .parse::<T>()
        .map_err(|_| invalid_value(reason))
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
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

fn invalid<T>(reason: &'static str) -> Result<T, StorageError> {
    Err(invalid_value(reason))
}

fn invalid_value(reason: &'static str) -> StorageError {
    StorageError::InvalidInput {
        field: "normalized_time",
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::validate_normalized_time;

    #[test]
    fn rejects_forbidden_terms_inside_safe_bracketed_zone() {
        for value in [
            "2026-07-01T10:00:00[America/Prompt]",
            "2026-07-01T10:00:00[America/Response]",
            "2026-07-01T10:00:00[America/Full_message]",
            "2026-07-01T10:00:00[America/Private_clinic_visit]",
            "2026-07-01T10:00:00[America/PromptText]",
            "2026-07-01T10:00:00[America/ResponseText]",
            "2026-07-01T10:00:00[America/FullMessage]",
            "2026-07-01T10:00:00[America/RawMessage]",
            "2026-07-01T10:00:00[America/MessageBody]",
            "2026-07-01T10:00:00[America/PrivateClinicVisit]",
        ] {
            // Given / When
            let error = validate_normalized_time(value);

            // Then
            assert!(error.is_err(), "{value}");
        }
    }

    #[test]
    fn rejects_variable_width_date_time_parts() {
        for value in [
            "2026-7-01T10:00:00Z",
            "2026-07-1T10:00:00Z",
            "2026-07-01T1:00:00Z",
            "2026-07-01T10:0:00Z",
            "2026-07-01T10:00:0Z",
        ] {
            // Given / When
            let error = validate_normalized_time(value);

            // Then
            assert!(error.is_err(), "{value}");
        }
    }

    #[test]
    fn accepts_safe_bracketed_zones() {
        for value in [
            "2026-07-01T10:00:00[America/Los_Angeles]",
            "2026-07-01T10:00:00[America/Argentina/Buenos_Aires]",
            "2026-07-01T10:00:00[Etc/UTC]",
        ] {
            // Given / When
            let result = validate_normalized_time(value);

            // Then
            assert!(result.is_ok(), "{value}");
        }
    }
}
