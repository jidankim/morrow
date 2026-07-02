use crate::normalized_time::validate_normalized_time;
use crate::types::{CandidateDraft, CandidateState, ExternalObjectMapping, QuietLogDraft};
use crate::StorageError;

pub(crate) fn transition_allowed(from_state: CandidateState, to_state: CandidateState) -> bool {
    match from_state {
        CandidateState::Queued => matches!(
            to_state,
            CandidateState::CreatingExternal
                | CandidateState::Suppressed
                | CandidateState::Rejected
                | CandidateState::Expired
                | CandidateState::Failed
        ),
        CandidateState::CreatingExternal => {
            matches!(
                to_state,
                CandidateState::Visible | CandidateState::Suppressed | CandidateState::Failed
            )
        }
        CandidateState::Visible => matches!(
            to_state,
            CandidateState::Approved
                | CandidateState::Rejected
                | CandidateState::Expired
                | CandidateState::Suppressed
                | CandidateState::Unknown
                | CandidateState::Failed
        ),
        CandidateState::Approved => matches!(
            to_state,
            CandidateState::Completed | CandidateState::Unknown
        ),
        CandidateState::Completed
        | CandidateState::Rejected
        | CandidateState::Expired
        | CandidateState::Suppressed
        | CandidateState::Unknown
        | CandidateState::Failed => false,
    }
}

pub(crate) fn validate_candidate_draft(draft: &CandidateDraft) -> Result<(), StorageError> {
    validate_text("chat_guid", &draft.chat_guid, 240)?;
    validate_text("anchor_message_guid", &draft.anchor_message_guid, 240)?;
    validate_text("title", &draft.title, 160)?;
    validate_normalized_time(&draft.normalized_time)?;
    validate_excerpt(&draft.evidence_excerpt)?;
    if (0..=1000).contains(&draft.confidence_millis) {
        Ok(())
    } else {
        Err(StorageError::InvalidInput {
            field: "confidence_millis",
            reason: "must be between 0 and 1000".to_owned(),
        })
    }
}

pub(crate) fn validate_quiet_log(draft: &QuietLogDraft) -> Result<(), StorageError> {
    validate_text("chat_guid", &draft.chat_guid, 240)?;
    validate_text("anchor_message_guid", &draft.anchor_message_guid, 240)?;
    validate_text("reason", &draft.reason, 240)?;
    validate_excerpt(&draft.excerpt)
}

pub(crate) fn validate_mapping(mapping: &ExternalObjectMapping) -> Result<(), StorageError> {
    validate_text("external_object_id", &mapping.external_object_id, 300)?;
    validate_text("external_source_id", &mapping.external_source_id, 180)
}

pub(crate) fn validate_text(
    field: &'static str,
    value: &str,
    max_len: usize,
) -> Result<(), StorageError> {
    if value.is_empty() {
        return Err(StorageError::InvalidInput {
            field,
            reason: "must not be empty".to_owned(),
        });
    }
    if value.len() > max_len {
        return Err(StorageError::InvalidInput {
            field,
            reason: format!("must be at most {max_len} bytes"),
        });
    }
    if value
        .chars()
        .any(|ch| matches!(ch, '\0' | '\u{1e}' | '\u{1f}'))
    {
        return Err(StorageError::InvalidInput {
            field,
            reason: "contains a reserved control character".to_owned(),
        });
    }
    Ok(())
}

fn validate_excerpt(value: &str) -> Result<(), StorageError> {
    validate_text("excerpt", value, 280)?;
    let lowered = value.to_ascii_lowercase();
    let header_like = lowered.contains("from:") && lowered.contains("to:");
    if value.lines().count() > 3 || header_like {
        Err(StorageError::PrivacyViolation {
            reason: "quiet/evidence storage accepts short excerpts only".to_owned(),
        })
    } else {
        Ok(())
    }
}
