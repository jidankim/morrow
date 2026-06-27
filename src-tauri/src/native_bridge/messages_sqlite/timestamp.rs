use morrow_messages::MessagesError;

const APPLE_EPOCH_UNIX_SECONDS: i64 = 978_307_200;
const NANOSECONDS_PER_SECOND: i64 = 1_000_000_000;
const MIN_NANOSECONDS_SINCE_2001: i64 = 10_000_000_000;

pub fn apple_timestamp_to_unix_seconds(value: i64) -> Result<i64, MessagesError> {
    if value < 0 {
        return Err(invalid_timestamp("must be non-negative"));
    }
    if value >= MIN_NANOSECONDS_SINCE_2001 {
        return value
            .checked_div(NANOSECONDS_PER_SECOND)
            .and_then(|seconds| seconds.checked_add(APPLE_EPOCH_UNIX_SECONDS))
            .ok_or_else(|| invalid_timestamp("nanosecond timestamp is out of range"));
    }
    value
        .checked_add(APPLE_EPOCH_UNIX_SECONDS)
        .ok_or_else(|| invalid_timestamp("second timestamp is out of range"))
}

fn invalid_timestamp(reason: &'static str) -> MessagesError {
    MessagesError::InvalidInput {
        field: "message_timestamp",
        reason: reason.to_owned(),
    }
}
