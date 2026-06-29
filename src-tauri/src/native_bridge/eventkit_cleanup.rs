use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

use super::DeleteMorrowDataError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProposedItemCleanupReceipt {
    pub(crate) calendar_items_deleted: u64,
    pub(crate) reminder_items_deleted: u64,
}

pub(crate) trait ProposedItemCleaner {
    fn cleanup_proposed_items(&self) -> Result<ProposedItemCleanupReceipt, DeleteMorrowDataError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct NoopProposedItemCleaner;

impl ProposedItemCleaner for NoopProposedItemCleaner {
    fn cleanup_proposed_items(&self) -> Result<ProposedItemCleanupReceipt, DeleteMorrowDataError> {
        Ok(ProposedItemCleanupReceipt {
            calendar_items_deleted: 0,
            reminder_items_deleted: 0,
        })
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct EventKitProposedItemCleaner;

impl ProposedItemCleaner for EventKitProposedItemCleaner {
    fn cleanup_proposed_items(&self) -> Result<ProposedItemCleanupReceipt, DeleteMorrowDataError> {
        cleanup_eventkit_proposed_items()
    }
}

#[cfg(target_os = "macos")]
fn cleanup_eventkit_proposed_items() -> Result<ProposedItemCleanupReceipt, DeleteMorrowDataError> {
    let mut raw = RawEventKitCleanupResult::default();
    // SAFETY: [Category 8 - FFI Boundary UB]
    // `raw` is a valid, aligned, writable pointer to a `repr(C)` struct whose
    // layout is mirrored by `MorrowEventKitCleanupResult` in
    // `eventkit_cleanup.m`. The Objective-C function writes only within that
    // struct and always NUL-terminates the fixed message buffer.
    unsafe { morrow_eventkit_cleanup_proposed_items(&mut raw) };

    if raw.ok == 0 {
        return Err(DeleteMorrowDataError::storage_unavailable(
            raw.message_string(),
        ));
    }

    Ok(ProposedItemCleanupReceipt {
        calendar_items_deleted: raw.calendar_items_deleted,
        reminder_items_deleted: raw.reminder_items_deleted,
    })
}

#[cfg(not(target_os = "macos"))]
fn cleanup_eventkit_proposed_items() -> Result<ProposedItemCleanupReceipt, DeleteMorrowDataError> {
    Err(DeleteMorrowDataError::storage_unavailable(
        "Morrow proposed item cleanup requires macOS EventKit",
    ))
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct RawEventKitCleanupResult {
    ok: c_int,
    calendar_items_deleted: u64,
    reminder_items_deleted: u64,
    message: [c_char; 512],
}

#[cfg(target_os = "macos")]
impl Default for RawEventKitCleanupResult {
    fn default() -> Self {
        Self {
            ok: 0,
            calendar_items_deleted: 0,
            reminder_items_deleted: 0,
            message: [0; 512],
        }
    }
}

#[cfg(target_os = "macos")]
impl RawEventKitCleanupResult {
    fn message_string(&self) -> String {
        // SAFETY: [Category 8 - FFI Boundary UB]
        // `message` is initialized to all zeroes before the FFI call, and the
        // Objective-C writer copies at most capacity - 1 bytes. A terminating
        // NUL is therefore always present within the fixed buffer.
        let message = unsafe { CStr::from_ptr(self.message.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        if message.is_empty() {
            "Morrow proposed item cleanup failed".to_owned()
        } else {
            message
        }
    }
}

#[cfg(target_os = "macos")]
extern "C" {
    fn morrow_eventkit_cleanup_proposed_items(result: *mut RawEventKitCleanupResult);
}

#[cfg(all(test, target_os = "macos"))]
extern "C" {
    fn morrow_eventkit_cleanup_note_has_morrow_metadata(notes: *const c_char) -> c_int;
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use std::ffi::CString;
    use std::os::raw::c_int;

    use super::morrow_eventkit_cleanup_note_has_morrow_metadata;

    #[test]
    fn cleanup_ownership_filter_requires_morrow_metadata() {
        let morrow = CString::new(
            "Review note\n[MORROW_METADATA_V1]\ncandidate_id=abc\nsource_id=def\n[/MORROW_METADATA_V1]",
        )
        .expect("fixture has no nul bytes");
        let manual = CString::new("manual user item").expect("fixture has no nul bytes");

        let morrow_result = native_note_has_morrow_metadata(&morrow);
        let manual_result = native_note_has_morrow_metadata(&manual);

        assert_eq!(morrow_result, 1);
        assert_eq!(manual_result, 0);
    }

    fn native_note_has_morrow_metadata(notes: &CString) -> c_int {
        // SAFETY: `notes` is a NUL-terminated CString that lives for the call.
        unsafe { morrow_eventkit_cleanup_note_has_morrow_metadata(notes.as_ptr()) }
    }
}
