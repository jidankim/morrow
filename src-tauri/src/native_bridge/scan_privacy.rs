use super::public_chat_id::{public_chat_id, public_message_id};
use super::scan::ScanSelectedChatsError;
use morrow_detection::SourceExcerptPolicy;
use morrow_messages::ChatGuid;
use morrow_storage::{CandidateDraft, CandidateId, QuietLogDraft};

const NATIVE_CANDIDATE_TITLE: &str = "Messages event candidate";
const HIDDEN_SOURCE_EXCERPT: &str = "Source excerpt hidden by settings.";

pub(super) fn privacy_safe_candidate(
    mut candidate: CandidateDraft,
    source_excerpts: SourceExcerptPolicy,
) -> Result<CandidateDraft, ScanSelectedChatsError> {
    candidate.chat_guid =
        public_chat_id(&ChatGuid::parse(&candidate.chat_guid).map_err(messages_error)?);
    candidate.anchor_message_guid = public_message_id(&candidate.anchor_message_guid);
    candidate.title = privacy_safe_candidate_title(&candidate.title);
    if source_excerpts == SourceExcerptPolicy::Hide {
        candidate.evidence_excerpt = HIDDEN_SOURCE_EXCERPT.to_owned();
    }
    Ok(candidate)
}

fn privacy_safe_candidate_title(raw: &str) -> String {
    let title = raw.trim();
    if title.is_empty() || title_has_private_marker(title) {
        NATIVE_CANDIDATE_TITLE.to_owned()
    } else {
        title.to_owned()
    }
}

fn title_has_private_marker(title: &str) -> bool {
    let lowered = title.to_ascii_lowercase();
    title.contains('@')
        || title.contains('+')
        || lowered.contains("private")
        || lowered.contains("raw-")
        || has_phone_like_digit_sequence(title, 7)
}

fn has_phone_like_digit_sequence(value: &str, threshold: usize) -> bool {
    let mut digits = 0;
    for ch in value.chars() {
        if ch.is_ascii_digit() {
            digits += 1;
            if digits >= threshold {
                return true;
            }
        } else if is_phone_title_char(ch) {
            continue;
        } else {
            digits = 0;
        }
    }
    false
}

fn is_phone_title_char(ch: char) -> bool {
    matches!(ch, '+' | '-' | '(' | ')' | '.' | ' ')
}

pub(super) fn privacy_safe_quiet_log(
    mut quiet_log: QuietLogDraft,
    source_excerpts: SourceExcerptPolicy,
) -> Result<QuietLogDraft, ScanSelectedChatsError> {
    quiet_log.chat_guid =
        public_chat_id(&ChatGuid::parse(&quiet_log.chat_guid).map_err(messages_error)?);
    quiet_log.anchor_message_guid = public_message_id(&quiet_log.anchor_message_guid);
    if source_excerpts == SourceExcerptPolicy::Hide {
        quiet_log.excerpt = HIDDEN_SOURCE_EXCERPT.to_owned();
    }
    Ok(quiet_log)
}

pub(super) fn candidate_ids_for_response(candidate_ids: &[CandidateId]) -> Vec<String> {
    candidate_ids
        .iter()
        .map(|candidate_id| candidate_id.as_str().to_owned())
        .collect()
}

fn messages_error(error: morrow_messages::MessagesError) -> ScanSelectedChatsError {
    ScanSelectedChatsError::Messages(error.to_string())
}
