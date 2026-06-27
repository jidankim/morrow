use crate::MessagesError;

pub(crate) fn validate_text(
    field: &'static str,
    value: &str,
    max_len: usize,
) -> Result<(), MessagesError> {
    if value.is_empty() {
        return Err(MessagesError::InvalidInput {
            field,
            reason: "must not be empty".to_owned(),
        });
    }
    if value.len() > max_len {
        return Err(MessagesError::InvalidInput {
            field,
            reason: format!("must be at most {max_len} bytes"),
        });
    }
    if value.chars().any(|ch| ch == '\0' || ch == '\u{1f}') {
        return Err(MessagesError::InvalidInput {
            field,
            reason: "contains a reserved control character".to_owned(),
        });
    }
    Ok(())
}

pub(crate) fn short_excerpt(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .take(120)
        .collect()
}
