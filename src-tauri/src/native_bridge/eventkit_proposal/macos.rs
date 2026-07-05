use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

// allow: SIZE_OK - This pre-existing macOS EventKit FFI adapter owns the
// Rust-side ABI structs, fixed-buffer decoding, and native symbol calls for
// one bridge surface. Splitting it during the T4 repair would risk changing
// repr(C) layout visibility or test/production cfg boundaries while the current
// gate blocker is a production-build cfg leak. Follow-up risk: extract the
// test fixtures and result mappers after T5-T7 replay QA locks the native ABI.

use morrow_calendar::EventRecord;

use super::{
    EventKitProposalError, EventKitProposalReceipt, EventKitReminderProposalError,
    EventKitReminderProposalReceipt, ReminderDueTimeZone, ReminderProposalRecord,
};

const ERROR_PERMISSION_DENIED: c_int = 1;
const ERROR_SOURCE_UNAVAILABLE: c_int = 2;
const ERROR_SAVE_FAILED: c_int = 3;
const ERROR_EMPTY_EVENT_IDENTIFIER: c_int = 4;
const ERROR_UNAVAILABLE: c_int = 5;
const TRUNCATED_EVENT_ID: c_int = 1;
const TRUNCATED_CALENDAR_ID: c_int = 2;
const TRUNCATED_SOURCE_ID: c_int = 3;
const TRUNCATED_REMINDER_ID: c_int = 1;
const TRUNCATED_LIST_ID: c_int = 2;
const TRUNCATED_REMINDER_SOURCE_ID: c_int = 3;
const METADATA_BEGIN: &str = "[MORROW_METADATA_V1]";
const METADATA_END: &str = "[/MORROW_METADATA_V1]";

pub(super) fn create_proposal_event(
    record: &EventRecord,
) -> Result<EventKitProposalReceipt, EventKitProposalError> {
    let title = cstring("title", &record.title)?;
    let notes = cstring("notes", &record.notes)?;
    let request = RawEventKitProposalRequest {
        title: title.as_ptr(),
        notes: notes.as_ptr(),
        start_unix: record.time_range.start_unix,
        end_unix: record.time_range.end_unix,
    };
    let mut raw = RawEventKitProposalResult::default();
    // SAFETY: [Category 8 - FFI Boundary UB]
    // `request` points to live NUL-terminated C strings for the duration
    // of the call, and `raw` is a valid, aligned, writable pointer to a
    // `repr(C)` result mirrored by `eventkit_proposal.m`. The Objective-C
    // function copies inputs synchronously and writes only within `raw`.
    unsafe { morrow_eventkit_create_proposal_event(&request, &mut raw) };
    raw.into_result()
}

pub(super) fn create_proposal_reminder(
    record: &ReminderProposalRecord,
) -> Result<EventKitReminderProposalReceipt, EventKitReminderProposalError> {
    let title = reminder_cstring("title", &record.title)?;
    let notes = reminder_cstring("notes", &record.notes)?;
    let metadata_candidate_id = reminder_cstring(
        "metadata_candidate_id",
        metadata_candidate_id_line(&record.notes)?,
    )?;
    let time_zone_name = match &record.due.time_zone {
        ReminderDueTimeZone::Named(name) => reminder_cstring("time_zone", name)?,
        ReminderDueTimeZone::Utc => reminder_cstring("time_zone", "UTC")?,
    };
    let request = RawEventKitReminderRequest {
        title: title.as_ptr(),
        notes: notes.as_ptr(),
        metadata_candidate_id: metadata_candidate_id.as_ptr(),
        due_year: record.due.year,
        due_month: i32::from(record.due.month),
        due_day: i32::from(record.due.day),
        has_due_time: 1,
        due_hour: i32::from(record.due.hour),
        due_minute: i32::from(record.due.minute),
        due_second: i32::from(record.due.second),
        timezone_name: time_zone_name.as_ptr(),
    };
    let mut raw = RawEventKitReminderResult::default();
    // SAFETY: [Category 8 - FFI Boundary UB]
    // `request` points to live NUL-terminated C strings for the duration
    // of the call, and `raw` is a valid, aligned, writable pointer to a
    // `repr(C)` result mirrored by `eventkit_proposal.m`. The Objective-C
    // function copies inputs synchronously and writes only within `raw`.
    unsafe { morrow_eventkit_create_proposal_reminder(&request, &mut raw) };
    raw.into_result()
}

fn cstring(field: &'static str, value: &str) -> Result<CString, EventKitProposalError> {
    CString::new(value).map_err(|_| EventKitProposalError::InvalidInput {
        field,
        reason: "value must not contain NUL bytes".to_owned(),
    })
}

fn reminder_cstring(
    field: &'static str,
    value: &str,
) -> Result<CString, EventKitReminderProposalError> {
    CString::new(value).map_err(|_| EventKitReminderProposalError::InvalidInput {
        field,
        reason: "value must not contain NUL bytes".to_owned(),
    })
}

fn metadata_candidate_id_line(notes: &str) -> Result<&str, EventKitReminderProposalError> {
    let end_index = notes
        .rfind(METADATA_END)
        .ok_or_else(missing_metadata_candidate_id)?;
    let begin_index = notes[..end_index]
        .rfind(METADATA_BEGIN)
        .ok_or_else(missing_metadata_candidate_id)?;
    notes[begin_index + METADATA_BEGIN.len()..end_index]
        .lines()
        .find(|line| line.starts_with("candidate_id="))
        .ok_or_else(missing_metadata_candidate_id)
}

fn missing_metadata_candidate_id() -> EventKitReminderProposalError {
    EventKitReminderProposalError::InvalidInput {
        field: "notes",
        reason: "missing Morrow metadata candidate id".to_owned(),
    }
}

#[repr(C)]
struct RawEventKitProposalRequest {
    title: *const c_char,
    notes: *const c_char,
    start_unix: i64,
    end_unix: i64,
}

#[repr(C)]
struct RawEventKitReminderRequest {
    title: *const c_char,
    notes: *const c_char,
    metadata_candidate_id: *const c_char,
    due_year: i32,
    due_month: i32,
    due_day: i32,
    has_due_time: c_int,
    due_hour: i32,
    due_minute: i32,
    due_second: i32,
    timezone_name: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct RawEventKitProposalResult {
    ok: c_int,
    error_code: c_int,
    truncated_field: c_int,
    event_id: [c_char; 256],
    calendar_id: [c_char; 256],
    source_id: [c_char; 256],
    message: [c_char; 512],
}

impl Default for RawEventKitProposalResult {
    fn default() -> Self {
        Self {
            ok: 0,
            error_code: 0,
            truncated_field: 0,
            event_id: [0; 256],
            calendar_id: [0; 256],
            source_id: [0; 256],
            message: [0; 512],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct RawEventKitReminderResult {
    ok: c_int,
    error_code: c_int,
    truncated_field: c_int,
    reminder_id: [c_char; 256],
    list_id: [c_char; 256],
    source_id: [c_char; 256],
    message: [c_char; 512],
}

impl Default for RawEventKitReminderResult {
    fn default() -> Self {
        Self {
            ok: 0,
            error_code: 0,
            truncated_field: 0,
            reminder_id: [0; 256],
            list_id: [0; 256],
            source_id: [0; 256],
            message: [0; 512],
        }
    }
}

impl RawEventKitProposalResult {
    pub(super) fn into_result(self) -> Result<EventKitProposalReceipt, EventKitProposalError> {
        if self.truncated_field != 0 {
            return Err(EventKitProposalError::SaveFailed {
                reason: truncated_identifier_message(self.truncated_field).to_owned(),
            });
        }
        if self.ok != 0 {
            return Ok(EventKitProposalReceipt {
                event_id: fixed_c_string(&self.event_id),
                calendar_id: fixed_c_string(&self.calendar_id),
                source_id: fixed_c_string(&self.source_id),
            });
        }
        let reason = self.message_string();
        match self.error_code {
            ERROR_PERMISSION_DENIED => Err(EventKitProposalError::PermissionDenied { reason }),
            ERROR_SOURCE_UNAVAILABLE => Err(EventKitProposalError::SourceUnavailable { reason }),
            ERROR_SAVE_FAILED => Err(EventKitProposalError::SaveFailed { reason }),
            ERROR_EMPTY_EVENT_IDENTIFIER => Err(EventKitProposalError::EmptyEventIdentifier),
            ERROR_UNAVAILABLE => Err(EventKitProposalError::Unavailable { reason }),
            _ => Err(EventKitProposalError::SaveFailed { reason }),
        }
    }

    fn message_string(&self) -> String {
        let message = fixed_c_string(&self.message);
        if message.is_empty() {
            "EventKit proposal bridge failed".to_owned()
        } else {
            message
        }
    }
}

impl RawEventKitReminderResult {
    pub(super) fn into_result(
        self,
    ) -> Result<EventKitReminderProposalReceipt, EventKitReminderProposalError> {
        if self.truncated_field != 0 {
            return Err(EventKitReminderProposalError::TruncatedIdentifier {
                field: truncated_reminder_identifier_field(self.truncated_field),
            });
        }
        if self.ok != 0 {
            return Ok(EventKitReminderProposalReceipt {
                reminder_id: fixed_c_string(&self.reminder_id),
                list_id: fixed_c_string(&self.list_id),
                source_id: fixed_c_string(&self.source_id),
            });
        }
        let reason = self.message_string();
        match self.error_code {
            ERROR_PERMISSION_DENIED => {
                Err(EventKitReminderProposalError::PermissionDenied { reason })
            }
            ERROR_SOURCE_UNAVAILABLE => {
                Err(EventKitReminderProposalError::SourceUnavailable { reason })
            }
            ERROR_SAVE_FAILED => Err(EventKitReminderProposalError::SaveFailed { reason }),
            ERROR_EMPTY_EVENT_IDENTIFIER => Err(EventKitReminderProposalError::EmptyIdentifier {
                field: "reminder_id",
            }),
            ERROR_UNAVAILABLE => Err(EventKitReminderProposalError::Unavailable { reason }),
            _ => Err(EventKitReminderProposalError::SaveFailed { reason }),
        }
    }

    fn message_string(&self) -> String {
        let message = fixed_c_string(&self.message);
        if message.is_empty() {
            "EventKit Reminders proposal bridge failed".to_owned()
        } else {
            message
        }
    }
}

fn truncated_identifier_message(field: c_int) -> &'static str {
    match field {
        TRUNCATED_EVENT_ID => "EventKit returned a truncated event identifier",
        TRUNCATED_CALENDAR_ID => "EventKit returned a truncated calendar identifier",
        TRUNCATED_SOURCE_ID => "EventKit returned a truncated source identifier",
        _ => "EventKit returned a truncated identifier",
    }
}

fn truncated_reminder_identifier_field(field: c_int) -> &'static str {
    match field {
        TRUNCATED_REMINDER_ID => "reminder_id",
        TRUNCATED_LIST_ID => "list_id",
        TRUNCATED_REMINDER_SOURCE_ID => "source_id",
        _ => "reminder_id",
    }
}

fn fixed_c_string(buffer: &[c_char]) -> String {
    // SAFETY: [Category 8 - FFI Boundary UB]
    // Every fixed string buffer is zero-initialized before the FFI call,
    // and the Objective-C writer copies at most capacity - 1 bytes. A NUL
    // terminator is therefore always present within the buffer.
    unsafe { CStr::from_ptr(buffer.as_ptr()) }
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
impl RawEventKitProposalResult {
    pub(super) fn success_for_test(
        event_id: &str,
        calendar_id: &str,
        source_id: &str,
        truncated_event_id: bool,
    ) -> Self {
        let mut raw = Self {
            ok: 1,
            ..Self::default()
        };
        write_fixed_for_test(&mut raw.event_id, event_id);
        write_fixed_for_test(&mut raw.calendar_id, calendar_id);
        write_fixed_for_test(&mut raw.source_id, source_id);
        if truncated_event_id {
            raw.truncated_field = TRUNCATED_EVENT_ID;
        }
        raw
    }
}

#[cfg(test)]
impl RawEventKitReminderResult {
    pub(super) fn failure_for_test(error_code: c_int, message: &str) -> Self {
        let mut raw = Self {
            error_code,
            ..Self::default()
        };
        write_fixed_for_test(&mut raw.message, message);
        raw
    }

    pub(super) fn success_for_test(
        reminder_id: &str,
        list_id: &str,
        source_id: &str,
        truncated_field: c_int,
    ) -> Self {
        let mut raw = Self {
            ok: 1,
            ..Self::default()
        };
        write_fixed_for_test(&mut raw.reminder_id, reminder_id);
        write_fixed_for_test(&mut raw.list_id, list_id);
        write_fixed_for_test(&mut raw.source_id, source_id);
        raw.truncated_field = truncated_field;
        raw
    }
}

#[cfg(test)]
fn write_fixed_for_test(buffer: &mut [c_char], value: &str) {
    let copy_len = buffer.len().saturating_sub(1).min(value.len());
    for (target, source) in buffer.iter_mut().zip(value.bytes()).take(copy_len) {
        *target = c_char::try_from(source).expect("test identifier is ascii");
    }
}

extern "C" {
    fn morrow_eventkit_create_proposal_event(
        request: *const RawEventKitProposalRequest,
        result: *mut RawEventKitProposalResult,
    );
    fn morrow_eventkit_create_proposal_reminder(
        request: *const RawEventKitReminderRequest,
        result: *mut RawEventKitReminderResult,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_truncated_native_reminder_list_id_without_raw_value() {
        // Given
        let raw = RawEventKitReminderResult::success_for_test(
            "reminder-native-id",
            "list-native-id-that-must-not-leak",
            "source-native-id",
            TRUNCATED_LIST_ID,
        );

        // When
        let error = raw.into_result().expect_err("truncated list id rejects");

        // Then
        assert_eq!(
            error,
            EventKitReminderProposalError::TruncatedIdentifier { field: "list_id" }
        );
        let message = error.to_string();
        assert!(message.contains("truncated list_id"), "{message}");
        assert!(!message.contains("list-native-id-that-must-not-leak"));
    }

    #[test]
    fn reads_candidate_id_from_morrow_metadata_block() -> Result<(), String> {
        // Given
        let notes = concat!(
            "candidate_id=wrong-user-note-line\n",
            "[MORROW_METADATA_V1]\n",
            "candidate_id=63616e6469646174652d72656d696e6465722d31\n",
            "source_id=6d6f72726f77\n",
            "[/MORROW_METADATA_V1]"
        );

        // When
        let candidate_line =
            metadata_candidate_id_line(notes).map_err(|error| error.to_string())?;

        // Then
        assert_eq!(
            candidate_line,
            "candidate_id=63616e6469646174652d72656d696e6465722d31"
        );
        Ok(())
    }

    #[test]
    fn maps_long_native_reminder_error_to_bounded_typed_error() {
        // Given
        let tail = "tail-that-must-not-appear";
        let raw = RawEventKitReminderResult::failure_for_test(
            ERROR_SAVE_FAILED,
            &format!("{}{}", "x".repeat(700), tail),
        );

        // When
        let error = raw.into_result().expect_err("native failure rejects");

        // Then
        match error {
            EventKitReminderProposalError::SaveFailed { reason } => {
                assert!(reason.len() <= 511, "{reason}");
                assert!(!reason.contains(tail), "{reason}");
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn compiles_reminder_ffi_shape_without_calling_eventkit() {
        let _ffi_entry: unsafe extern "C" fn(
            *const RawEventKitReminderRequest,
            *mut RawEventKitReminderResult,
        ) = morrow_eventkit_create_proposal_reminder;
        let _bridge_entry: fn(
            &ReminderProposalRecord,
        ) -> Result<
            EventKitReminderProposalReceipt,
            EventKitReminderProposalError,
        > = create_proposal_reminder;
    }
}
