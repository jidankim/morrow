use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;

use morrow_calendar::EventRecord;

use super::{
    EventKitProposalError, EventKitProposalReceipt, EventKitReminderReceipt, ReminderRecord,
};

const ERROR_PERMISSION_DENIED: c_int = 1;
const ERROR_SOURCE_UNAVAILABLE: c_int = 2;
const ERROR_SAVE_FAILED: c_int = 3;
const ERROR_EMPTY_EVENT_IDENTIFIER: c_int = 4;
const ERROR_UNAVAILABLE: c_int = 5;
const TRUNCATED_EVENT_ID: c_int = 1;
const TRUNCATED_CALENDAR_ID: c_int = 2;
const TRUNCATED_SOURCE_ID: c_int = 3;

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
    record: &ReminderRecord,
) -> Result<EventKitReminderReceipt, EventKitProposalError> {
    let title = cstring("title", &record.title)?;
    let notes = cstring("notes", &record.notes)?;
    let timezone_name = record
        .due
        .timezone_name
        .as_deref()
        .map(|value| cstring("timezone_name", value))
        .transpose()?;
    let due_time = record.due.time;
    let request = RawEventKitReminderRequest {
        title: title.as_ptr(),
        notes: notes.as_ptr(),
        due_year: record.due.date.year,
        due_month: c_int::from(record.due.date.month),
        due_day: c_int::from(record.due.date.day),
        has_due_time: if due_time.is_some() { 1 } else { 0 },
        due_hour: due_time.map_or(0, |time| c_int::from(time.hour)),
        due_minute: due_time.map_or(0, |time| c_int::from(time.minute)),
        timezone_name: timezone_name
            .as_ref()
            .map_or(ptr::null(), |value| value.as_ptr()),
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
    due_year: c_int,
    due_month: c_int,
    due_day: c_int,
    has_due_time: c_int,
    due_hour: c_int,
    due_minute: c_int,
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
    pub(super) fn into_result(self) -> Result<EventKitReminderReceipt, EventKitProposalError> {
        if self.truncated_field != 0 {
            return Err(EventKitProposalError::SaveFailed {
                reason: truncated_identifier_message(self.truncated_field).to_owned(),
            });
        }
        if self.ok != 0 {
            return Ok(EventKitReminderReceipt {
                reminder_id: fixed_c_string(&self.reminder_id),
                list_id: fixed_c_string(&self.list_id),
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

fn truncated_identifier_message(field: c_int) -> &'static str {
    match field {
        TRUNCATED_EVENT_ID => "EventKit returned a truncated event identifier",
        TRUNCATED_CALENDAR_ID => "EventKit returned a truncated calendar identifier",
        TRUNCATED_SOURCE_ID => "EventKit returned a truncated source identifier",
        _ => "EventKit returned a truncated identifier",
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
