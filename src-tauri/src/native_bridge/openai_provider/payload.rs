use morrow_messages::MessageEvidence;
use serde::Serialize;
use serde_json::{json, Value};

use super::{OpenAiProviderError, OPENAI_MODEL};

const MAX_EVIDENCE_MESSAGES: usize = 20;
const MAX_EXCERPT_BYTES: usize = 120;
const MAX_SERIALIZED_EVIDENCE_BYTES: usize = 4 * 1024;

#[derive(Debug, Serialize)]
struct EvidencePayload {
    selected_chat_evidence: Vec<AllowedEvidence>,
}

#[derive(Debug, Serialize)]
struct AllowedEvidence {
    message_guid: String,
    timestamp: i64,
    participant_count: u16,
    tapback_signal: bool,
    excerpt: String,
    evidence_pointer: String,
}

pub(super) fn request_body(evidence: &[MessageEvidence]) -> Result<Value, OpenAiProviderError> {
    let payload = EvidencePayload {
        selected_chat_evidence: evidence
            .iter()
            .take(MAX_EVIDENCE_MESSAGES)
            .enumerate()
            .map(|(index, message)| evidence_record(index, message))
            .collect(),
    };
    let evidence_text =
        serde_json::to_string(&payload).map_err(|_| OpenAiProviderError::InvalidResponse {
            reason: "evidence serialization failed",
        })?;
    if evidence_text.len() > MAX_SERIALIZED_EVIDENCE_BYTES {
        return Err(OpenAiProviderError::EvidenceTooLarge);
    }
    Ok(json!({
        "model": OPENAI_MODEL,
        "input": [{
            "role": "user",
            "content": [{ "type": "input_text", "text": evidence_text }]
        }],
        "text": {
            "format": {
                "type": "json_schema",
                "name": "morrow_provider_candidate",
                "strict": true,
                "schema": candidate_schema()
            }
        }
    }))
}

fn evidence_record(index: usize, message: &MessageEvidence) -> AllowedEvidence {
    AllowedEvidence {
        message_guid: message.message_guid.as_str().to_owned(),
        timestamp: message.timestamp.as_i64(),
        participant_count: message.participant_count,
        tapback_signal: message.tapback_signal,
        excerpt: bounded_excerpt(&redact_secrets(&message.excerpt)),
        evidence_pointer: format!("evidence://selected/{index}"),
    }
}

fn candidate_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "kind",
            "title",
            "confidence_millis",
            "normalized_time",
            "anchor_message_guid",
            "evidence_message_guids"
        ],
        "properties": {
            "kind": {
                "type": "string",
                "enum": [
                    "calendar_event",
                    "task_reminder",
                    "event_update",
                    "event_reschedule",
                    "event_cancellation",
                    "reminder_update",
                    "reminder_reschedule",
                    "reminder_cancellation"
                ]
            },
            "title": { "type": "string" },
            "confidence_millis": { "type": "integer", "minimum": 0, "maximum": 1000 },
            "normalized_time": { "type": "string" },
            "anchor_message_guid": { "type": "string" },
            "evidence_message_guids": {
                "type": "array",
                "minItems": 1,
                "items": { "type": "string" }
            }
        }
    })
}

fn redact_secrets(value: &str) -> String {
    redact_phone_like(&redact_email_like(value))
}

fn redact_email_like(value: &str) -> String {
    value
        .split_whitespace()
        .map(|token| {
            if looks_like_email(token) {
                "[email]".to_owned()
            } else {
                token.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn looks_like_email(token: &str) -> bool {
    let trimmed = token.trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
    let Some((local, domain)) = trimmed.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.ends_with('.')
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
