use time::{
    format_description::well_known::{Iso8601, Rfc3339},
    OffsetDateTime, PrimitiveDateTime, UtcOffset,
};
use time_tz::{timezones, OffsetResult, PrimitiveDateTimeExt};

use super::{external_proposal_error, ScanSelectedChatsError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NormalizedProposalTime {
    pub(super) unix_timestamp: i64,
    pub(super) reminder_due_components: ReminderDueComponents,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReminderDueComponents {
    pub(super) year: i32,
    pub(super) month: u8,
    pub(super) day: u8,
    pub(super) hour: u8,
    pub(super) minute: u8,
    pub(super) second: u8,
    pub(super) time_zone: ReminderDueTimeZone,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ReminderDueTimeZone {
    Named(String),
    Utc,
}

pub(super) fn parse_normalized_time(
    value: &str,
) -> Result<NormalizedProposalTime, ScanSelectedChatsError> {
    match value.split_once('[') {
        Some((local_time, timezone_with_bracket)) => {
            let timezone_name = timezone_with_bracket
                .strip_suffix(']')
                .ok_or_else(|| external_proposal_error("invalid normalized_time timezone"))?;
            let local_datetime =
                PrimitiveDateTime::parse(local_time, &Iso8601::DEFAULT).map_err(|error| {
                    external_proposal_error(format!("invalid normalized_time: {error}"))
                })?;
            let timezone = timezones::get_by_name(timezone_name)
                .ok_or_else(|| external_proposal_error("unsupported normalized_time timezone"))?;
            match local_datetime.assume_timezone(timezone) {
                OffsetResult::Some(datetime) => Ok(NormalizedProposalTime {
                    unix_timestamp: datetime.unix_timestamp(),
                    reminder_due_components: due_components_from_local(
                        local_datetime,
                        timezone_name,
                    ),
                }),
                OffsetResult::Ambiguous(_, _) | OffsetResult::None => Err(external_proposal_error(
                    "ambiguous or invalid normalized_time timezone",
                )),
            }
        }
        None => {
            let datetime = OffsetDateTime::parse(value, &Rfc3339).map_err(|error| {
                external_proposal_error(format!("invalid normalized_time: {error}"))
            })?;
            let utc_datetime = datetime.to_offset(UtcOffset::UTC);
            Ok(NormalizedProposalTime {
                unix_timestamp: datetime.unix_timestamp(),
                reminder_due_components: due_components_from_utc(utc_datetime),
            })
        }
    }
}

fn due_components_from_local(
    datetime: PrimitiveDateTime,
    timezone_name: &str,
) -> ReminderDueComponents {
    ReminderDueComponents {
        year: datetime.year(),
        month: u8::from(datetime.month()),
        day: datetime.day(),
        hour: datetime.hour(),
        minute: datetime.minute(),
        second: datetime.second(),
        time_zone: ReminderDueTimeZone::Named(timezone_name.to_owned()),
    }
}

fn due_components_from_utc(datetime: OffsetDateTime) -> ReminderDueComponents {
    ReminderDueComponents {
        year: datetime.year(),
        month: u8::from(datetime.month()),
        day: datetime.day(),
        hour: datetime.hour(),
        minute: datetime.minute(),
        second: datetime.second(),
        time_zone: ReminderDueTimeZone::Utc,
    }
}

#[cfg(test)]
#[path = "normalized_time/replay_failure_tests.rs"]
mod replay_failure_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_normalized_time_preserves_named_timezone_due_components() {
        // Given
        let normalized_time = "2026-07-15T14:00:00[Asia/Seoul]";

        // When
        let parsed = parse_normalized_time(normalized_time).expect("named timezone parses");

        // Then
        assert_eq!(parsed.unix_timestamp, 1_784_091_600);
        assert_eq!(
            parsed.reminder_due_components,
            ReminderDueComponents {
                year: 2026,
                month: 7,
                day: 15,
                hour: 14,
                minute: 0,
                second: 0,
                time_zone: ReminderDueTimeZone::Named("Asia/Seoul".to_owned()),
            }
        );
    }

    #[test]
    fn parse_normalized_time_uses_utc_due_components_for_zulu_time() {
        // Given
        let normalized_time = "2026-07-15T05:00:00Z";

        // When
        let parsed = parse_normalized_time(normalized_time).expect("zulu time parses");

        // Then
        assert_eq!(parsed.unix_timestamp, 1_784_091_600);
        assert_eq!(
            parsed.reminder_due_components,
            ReminderDueComponents {
                year: 2026,
                month: 7,
                day: 15,
                hour: 5,
                minute: 0,
                second: 0,
                time_zone: ReminderDueTimeZone::Utc,
            }
        );
    }

    #[test]
    fn parse_normalized_time_rejects_unknown_timezone_without_echoing_value() {
        // Given
        let normalized_time = "2026-07-15T14:00:00[Mars/Olympus_Mons]";

        // When
        let error = parse_normalized_time(normalized_time).expect_err("timezone rejects");

        // Then
        assert_external_error_contains(&error, "unsupported normalized_time timezone");
        assert_external_error_excludes(&error, "Mars/Olympus_Mons");
    }

    #[test]
    fn parse_normalized_time_rejects_ambiguous_named_timezone() {
        // Given
        let normalized_time = "2026-11-01T01:30:00[America/Los_Angeles]";

        // When
        let error = parse_normalized_time(normalized_time).expect_err("ambiguous time rejects");

        // Then
        assert_external_error_contains(&error, "ambiguous or invalid normalized_time timezone");
        assert_external_error_excludes(&error, "America/Los_Angeles");
    }

    #[test]
    fn parse_normalized_time_rejects_invalid_named_timezone_local_time() {
        // Given
        let normalized_time = "2026-03-08T02:30:00[America/Los_Angeles]";

        // When
        let error = parse_normalized_time(normalized_time).expect_err("invalid time rejects");

        // Then
        assert_external_error_contains(&error, "ambiguous or invalid normalized_time timezone");
        assert_external_error_excludes(&error, "America/Los_Angeles");
    }

    fn assert_external_error_contains(error: &ScanSelectedChatsError, expected: &str) {
        match error {
            ScanSelectedChatsError::ExternalProposal(message) => {
                assert!(message.contains(expected), "{message}");
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    fn assert_external_error_excludes(error: &ScanSelectedChatsError, unexpected: &str) {
        match error {
            ScanSelectedChatsError::ExternalProposal(message) => {
                assert!(!message.contains(unexpected), "{message}");
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
