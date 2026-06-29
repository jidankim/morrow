use morrow_messages::MessagesError;

use super::hex;

#[cfg(target_os = "macos")]
use std::ffi::CStr;
#[cfg(target_os = "macos")]
use std::os::raw::{c_char, c_int};

#[cfg(target_os = "macos")]
const ERROR_DECODE_FAILED: c_int = 1;
#[cfg(target_os = "macos")]
const ERROR_TEXT_TRUNCATED: c_int = 2;

pub(super) fn text_from_hex(value: &str) -> Result<Option<String>, MessagesError> {
    let bytes = hex::bytes(value, "attributed body")?;
    if bytes.is_empty() {
        return Ok(None);
    }
    decode_attributed_body(&bytes)
}

#[cfg(target_os = "macos")]
fn decode_attributed_body(bytes: &[u8]) -> Result<Option<String>, MessagesError> {
    let mut raw = RawAttributedBodyResult::default();
    // SAFETY: [Category 8 - FFI Boundary UB]
    // `bytes.as_ptr()` is non-null for the non-empty slice passed here and
    // remains valid for `bytes.len()` bytes during this synchronous call.
    // `raw` is a valid, aligned, writable result mirrored by
    // `messages_attributed_body.m`, which copies output into fixed buffers.
    unsafe { morrow_messages_attributed_body_text(bytes.as_ptr(), bytes.len(), &mut raw) };
    raw.into_text()
}

#[cfg(not(target_os = "macos"))]
fn decode_attributed_body(_bytes: &[u8]) -> Result<Option<String>, MessagesError> {
    Ok(None)
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
struct RawAttributedBodyResult {
    ok: c_int,
    error_code: c_int,
    text: [c_char; 4096],
    message: [c_char; 256],
}

#[cfg(target_os = "macos")]
impl Default for RawAttributedBodyResult {
    fn default() -> Self {
        Self {
            ok: 0,
            error_code: 0,
            text: [0; 4096],
            message: [0; 256],
        }
    }
}

#[cfg(target_os = "macos")]
impl RawAttributedBodyResult {
    fn into_text(self) -> Result<Option<String>, MessagesError> {
        if self.ok != 0 {
            return Ok(Some(fixed_c_string(&self.text)));
        }
        match self.error_code {
            ERROR_DECODE_FAILED => Ok(None),
            ERROR_TEXT_TRUNCATED => Err(unavailable(self.message_string())),
            _ => Err(unavailable(self.message_string())),
        }
    }

    fn message_string(&self) -> String {
        let message = fixed_c_string(&self.message);
        if message.is_empty() {
            "attributed body decode failed".to_owned()
        } else {
            message
        }
    }
}

#[cfg(target_os = "macos")]
fn fixed_c_string(buffer: &[c_char]) -> String {
    // SAFETY: [Category 8 - FFI Boundary UB]
    // Objective-C zero-initializes every result buffer and writes at most
    // capacity - 1 bytes, so a NUL terminator is always present.
    unsafe { CStr::from_ptr(buffer.as_ptr()) }
        .to_string_lossy()
        .into_owned()
}

#[cfg(target_os = "macos")]
extern "C" {
    fn morrow_messages_attributed_body_text(
        bytes: *const u8,
        length: usize,
        result: *mut RawAttributedBodyResult,
    );
}

fn unavailable(reason: impl Into<String>) -> MessagesError {
    MessagesError::NativeUnavailable {
        reason: reason.into(),
    }
}
