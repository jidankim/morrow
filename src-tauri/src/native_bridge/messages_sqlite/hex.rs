use morrow_messages::MessagesError;

pub(super) fn text(value: &str) -> Result<String, MessagesError> {
    String::from_utf8(bytes(value, "hex text")?)
        .map_err(|error| unavailable(format!("sqlite returned non-utf8 text: {error}")))
}

pub(super) fn bytes(value: &str, field: &'static str) -> Result<Vec<u8>, MessagesError> {
    let bytes = value.as_bytes();
    if bytes.len() % 2 != 0 {
        return Err(unavailable(format!("sqlite returned odd-length {field}")));
    }
    let mut decoded = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let high = nibble(pair[0])
            .ok_or_else(|| unavailable(format!("sqlite returned invalid {field}")))?;
        let low = nibble(pair[1])
            .ok_or_else(|| unavailable(format!("sqlite returned invalid {field}")))?;
        decoded.push((high << 4) | low);
    }
    Ok(decoded)
}

const fn nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn unavailable(reason: impl Into<String>) -> MessagesError {
    MessagesError::NativeUnavailable {
        reason: reason.into(),
    }
}
