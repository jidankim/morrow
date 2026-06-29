use crate::native_bridge::public_chat_id::{public_chat_id, public_message_id};

use morrow_messages::ChatGuid;
use morrow_storage::{CandidateDraft, CandidateId, QuietLogDraft};

use super::{messages_error, ScanSelectedChatsError};

const NATIVE_CANDIDATE_TITLE: &str = "Messages event candidate";
const HIDDEN_SOURCE_EXCERPT: &str = "Source excerpt hidden by settings.";

pub(super) fn privacy_safe_candidate(
    mut candidate: CandidateDraft,
) -> Result<CandidateDraft, ScanSelectedChatsError> {
    candidate.chat_guid =
        public_chat_id(&ChatGuid::parse(&candidate.chat_guid).map_err(messages_error)?);
    candidate.anchor_message_guid = public_message_id(&candidate.anchor_message_guid);
    candidate.title = privacy_safe_title(&candidate.title);
    candidate.evidence_excerpt = HIDDEN_SOURCE_EXCERPT.to_owned();
    Ok(candidate)
}

fn privacy_safe_title(raw: &str) -> String {
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
        || has_digit_run(title, 7)
}

fn has_digit_run(value: &str, threshold: usize) -> bool {
    let mut run = 0;
    for ch in value.chars() {
        if ch.is_ascii_digit() {
            run += 1;
            if run >= threshold {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

pub(super) fn privacy_safe_quiet_log(
    mut quiet_log: QuietLogDraft,
) -> Result<QuietLogDraft, ScanSelectedChatsError> {
    quiet_log.chat_guid =
        public_chat_id(&ChatGuid::parse(&quiet_log.chat_guid).map_err(messages_error)?);
    quiet_log.anchor_message_guid = public_message_id(&quiet_log.anchor_message_guid);
    quiet_log.excerpt = HIDDEN_SOURCE_EXCERPT.to_owned();
    Ok(quiet_log)
}

pub(super) fn candidate_ids_for_response(candidate_ids: &[CandidateId]) -> Vec<String> {
    candidate_ids
        .iter()
        .map(|candidate_id| candidate_id.as_str().to_owned())
        .collect()
}
