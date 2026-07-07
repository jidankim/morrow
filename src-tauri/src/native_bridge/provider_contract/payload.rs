use morrow_messages::MessageEvidence;
use serde::Serialize;

use super::ProviderContractError;

pub(super) const MAX_EVIDENCE_MESSAGES: usize = 20;
const MAX_EXCERPT_BYTES: usize = 120;
const MAX_SERIALIZED_EVIDENCE_BYTES: usize = 4 * 1024;

#[derive(Debug, Serialize)]
struct EvidencePayload {
    selected_chat_evidence: Vec<AllowedEvidence>,
}

#[derive(Debug, Serialize)]
struct AllowedEvidence {
    evidence_id: String,
    timestamp: i64,
    participant_count: u16,
    tapback_signal: bool,
    excerpt: String,
}

pub(crate) fn evidence_payload_text(
    evidence: &[MessageEvidence],
) -> Result<String, ProviderContractError> {
    let payload = EvidencePayload {
        selected_chat_evidence: evidence
            .iter()
            .take(MAX_EVIDENCE_MESSAGES)
            .enumerate()
            .map(|(index, message)| evidence_record(index, message))
            .collect(),
    };
    let evidence_text = serde_json::to_string(&payload)
        .map_err(|_| ProviderContractError::EvidenceSerialization)?;
    if evidence_text.len() > MAX_SERIALIZED_EVIDENCE_BYTES {
        return Err(ProviderContractError::EvidenceTooLarge);
    }
    Ok(evidence_text)
}

fn evidence_record(index: usize, message: &MessageEvidence) -> AllowedEvidence {
    AllowedEvidence {
        evidence_id: format!("evidence://selected/{index}"),
        timestamp: message.timestamp.as_i64(),
        participant_count: message.participant_count,
        tapback_signal: message.tapback_signal,
        excerpt: bounded_excerpt(&redact_sensitive_text(&message.excerpt)),
    }
}

fn redact_sensitive_text(value: &str) -> String {
    redact_phone_like(&redact_token_like(&redact_handle_like(&redact_email_like(
        value,
    ))))
}

fn redact_email_like(value: &str) -> String {
    value
        .split_whitespace()
        .map(redact_email_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_email_token(token: &str) -> String {
    if looks_like_email(token) {
        "[email]".to_owned()
    } else {
        token.to_owned()
    }
}

fn looks_like_email(token: &str) -> bool {
    let trimmed = token.trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
    let Some((local, domain)) = trimmed.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.ends_with('.')
}

fn redact_handle_like(value: &str) -> String {
    value
        .split_whitespace()
        .map(redact_handle_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_handle_token(token: &str) -> String {
    let trimmed = token.trim_matches(|ch: char| !is_handle_body_char(ch) && ch != '@');
    if looks_like_handle(trimmed) {
        token.replacen(trimmed, "[handle]", 1)
    } else {
        token.to_owned()
    }
}

fn looks_like_handle(token: &str) -> bool {
    let Some(rest) = token.strip_prefix('@') else {
        return false;
    };
    !rest.is_empty() && rest.chars().all(is_handle_body_char)
}

fn is_handle_body_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.'
}

fn redact_token_like(value: &str) -> String {
    value
        .split_whitespace()
        .map(redact_secret_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_secret_token(token: &str) -> String {
    if looks_like_secret_token(token) {
        "[token]".to_owned()
    } else {
        token.to_owned()
    }
}

fn looks_like_secret_token(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    token.starts_with("sk-")
        || lower.contains("sk-proj-")
        || lower.contains("api_key=")
        || lower.contains("access_token=")
        || lower.contains("codex_access_token")
        || lower.contains("codex-token")
}

fn redact_phone_like(value: &str) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    let mut output = String::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index].is_ascii_digit() || chars[index] == '+' {
            index = push_phone_or_original(&chars, index, &mut output);
        } else {
            output.push(chars[index]);
            index += 1;
        }
    }
    output
}

fn push_phone_or_original(chars: &[char], start: usize, output: &mut String) -> usize {
    let mut index = start;
    let mut digits = 0;
    while index < chars.len() && is_phone_char(chars[index]) {
        if chars[index].is_ascii_digit() {
            digits += 1;
        }
        index += 1;
    }
    if digits >= 7 {
        output.push_str("[phone]");
    } else {
        output.extend(chars[start..index].iter());
    }
    index
}

fn is_phone_char(ch: char) -> bool {
    ch.is_ascii_digit() || matches!(ch, '+' | '-' | '(' | ')' | '.' | ' ')
}

fn bounded_excerpt(value: &str) -> String {
    let mut output = String::new();
    for ch in value.chars() {
        if output.len() + ch.len_utf8() > MAX_EXCERPT_BYTES {
            break;
        }
        output.push(ch);
    }
    output
}
