use morrow_messages::{ChatGuid, MessageEvidence, MessageGuid, MessageTimestamp};
use morrow_storage::{CandidateDraft, CandidateKind, QuietLogDraft};

pub(super) fn message(
    chat_guid: &str,
    message_guid: &str,
    excerpt: &str,
) -> Result<MessageEvidence, String> {
    Ok(MessageEvidence {
        chat_guid: ChatGuid::parse(chat_guid).map_err(|error| error.to_string())?,
        message_guid: MessageGuid::parse(message_guid).map_err(|error| error.to_string())?,
        timestamp: MessageTimestamp::new(1_782_352_400).map_err(|error| error.to_string())?,
        participant_count: 3,
        tapback_signal: true,
        excerpt: excerpt.to_owned(),
        evidence_pointer: "fixture-pointer".to_owned(),
    })
}

pub(super) fn candidate(chat_guid: &str, message_guid: &str, title: &str) -> CandidateDraft {
    candidate_with_kind(CandidateKind::CalendarEvent, chat_guid, message_guid, title)
}

pub(super) fn candidate_with_kind(
    kind: CandidateKind,
    chat_guid: &str,
    message_guid: &str,
    title: &str,
) -> CandidateDraft {
    CandidateDraft {
        kind,
        chat_guid: chat_guid.to_owned(),
        anchor_message_guid: message_guid.to_owned(),
        title: title.to_owned(),
        confidence_millis: 980,
        normalized_time: "2026-07-15T14:00:00+09:00".to_owned(),
        evidence_excerpt: "private source excerpt".to_owned(),
        observed_at: 1_782_352_400,
    }
}

pub(super) fn quiet_log(chat_guid: &str, message_guid: &str, excerpt: &str) -> QuietLogDraft {
    QuietLogDraft {
        chat_guid: chat_guid.to_owned(),
        anchor_message_guid: message_guid.to_owned(),
        reason: "no actionable event".to_owned(),
        excerpt: excerpt.to_owned(),
        created_at: 1_782_352_400,
        provider_diagnostic: None,
    }
}

pub(super) fn assert_public_ids_without_raw_values(
    chat_guid: &str,
    anchor_message_guid: &str,
    forbidden: &[&str],
) {
    assert!(chat_guid.starts_with("messages-chat-"), "{chat_guid}");
    assert!(
        anchor_message_guid.starts_with("messages-message-"),
        "{anchor_message_guid}"
    );
    for raw in forbidden {
        assert!(
            !chat_guid.contains(raw) && !anchor_message_guid.contains(raw),
            "planned public IDs leaked {raw}: {chat_guid} {anchor_message_guid}"
        );
    }
}
