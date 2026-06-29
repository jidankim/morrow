use super::public_chat_id::{public_chat_id, public_message_id};
use super::scan::ScanSelectedChatsError;
use morrow_messages::ChatGuid;
use morrow_storage::{CandidateDraft, CandidateId, QuietLogDraft};

const NATIVE_CANDIDATE_TITLE: &str = "Messages event candidate";
const HIDDEN_SOURCE_EXCERPT: &str = "Source excerpt hidden by settings.";

pub(super) fn privacy_safe_candidate(
    mut candidate: CandidateDraft,
) -> Result<CandidateDraft, ScanSelectedChatsError> {
    candidate.chat_guid =
        public_chat_id(&ChatGuid::parse(&candidate.chat_guid).map_err(messages_error)?);
    candidate.anchor_message_guid = public_message_id(&candidate.anchor_message_guid);
    candidate.title = NATIVE_CANDIDATE_TITLE.to_owned();
    candidate.evidence_excerpt = HIDDEN_SOURCE_EXCERPT.to_owned();
    Ok(candidate)
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

fn messages_error(error: morrow_messages::MessagesError) -> ScanSelectedChatsError {
    ScanSelectedChatsError::Messages(error.to_string())
}
