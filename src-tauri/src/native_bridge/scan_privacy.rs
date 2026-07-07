use super::public_chat_id::{public_chat_id, public_message_id};
use super::scan::ScanSelectedChatsError;
use morrow_detection::SourceExcerptPolicy;
use morrow_messages::ChatGuid;
use morrow_storage::{
    privacy_safe_native_scan_title, CandidateDraft, CandidateId, CandidateKind, QuietLogDraft,
    PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE,
};

const HIDDEN_SOURCE_EXCERPT: &str = "Source excerpt hidden by settings.";
const NATIVE_SCAN_REMINDER_TITLE: &str = "Messages reminder candidate";

pub(super) fn privacy_safe_candidate(
    mut candidate: CandidateDraft,
    source_excerpts: SourceExcerptPolicy,
) -> Result<CandidateDraft, ScanSelectedChatsError> {
    candidate.chat_guid =
        public_chat_id(&ChatGuid::parse(&candidate.chat_guid).map_err(messages_error)?);
    candidate.anchor_message_guid = public_message_id(&candidate.anchor_message_guid);
    candidate.title = privacy_safe_candidate_title(candidate.kind, &candidate.title);
    if source_excerpts == SourceExcerptPolicy::Hide {
        candidate.evidence_excerpt = HIDDEN_SOURCE_EXCERPT.to_owned();
    }
    Ok(candidate)
}

fn privacy_safe_candidate_title(kind: CandidateKind, raw: &str) -> String {
    let fallback = match kind {
        CandidateKind::TaskReminder => NATIVE_SCAN_REMINDER_TITLE,
        CandidateKind::CalendarEvent
        | CandidateKind::EventUpdate
        | CandidateKind::EventReschedule
        | CandidateKind::EventCancellation
        | CandidateKind::ReminderUpdate
        | CandidateKind::ReminderReschedule
        | CandidateKind::ReminderCancellation => PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE,
    };
    privacy_safe_native_scan_title(raw, fallback)
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
