use crate::sqlite_cli::{row_value, sql_text};
use crate::types::{CandidateKind, CandidateState, ExternalSource, ReminderProposalPayload};
use crate::{CandidateId, StorageError, Store};

const SELECTED_MESSAGES_SOURCE_ID: &str = "morrow-selected-messages";
const NATIVE_SCAN_TITLE: &str = "Messages reminder candidate";

impl Store {
    pub fn reminder_proposal_payloads(
        &self,
        candidate_ids: &[CandidateId],
    ) -> Result<Vec<ReminderProposalPayload>, StorageError> {
        let mut payloads = Vec::new();
        for candidate_id in candidate_ids {
            if let Some(payload) = self.reminder_proposal_payload(candidate_id)? {
                payloads.push(payload);
            }
        }
        Ok(payloads)
    }

    fn reminder_proposal_payload(
        &self,
        candidate_id: &CandidateId,
    ) -> Result<Option<ReminderProposalPayload>, StorageError> {
        let sql = format!(
            "SELECT c.id, c.kind, c.normalized_time, c.title
             FROM candidates c
             LEFT JOIN external_object_mappings m
               ON m.candidate_id = c.id AND m.source = {source}
             WHERE c.id = {id}
               AND c.state = {state}
               AND c.kind = {kind}
               AND m.id IS NULL;",
            id = sql_text(candidate_id.as_str())?,
            source = sql_text(ExternalSource::Reminders.as_str())?,
            state = sql_text(CandidateState::CreatingExternal.as_str())?,
            kind = sql_text(CandidateKind::TaskReminder.as_str())?,
        );
        let rows = self.sqlite.query_rows(&sql)?;
        let Some(row) = rows.first() else {
            return Ok(None);
        };
        Ok(Some(ReminderProposalPayload {
            candidate_id: CandidateId::from_storage(row_value(row, 0, "reminder_payload.id")?)?,
            kind: CandidateKind::parse(row_value(row, 1, "reminder_payload.kind")?)?,
            normalized_time: row_value(row, 2, "reminder_payload.normalized_time")?.to_owned(),
            title: reminder_payload_title(row_value(row, 3, "reminder_payload.title")?),
            source_id: SELECTED_MESSAGES_SOURCE_ID.to_owned(),
        }))
    }
}

fn reminder_payload_title(raw: &str) -> String {
    let title = raw.trim();
    if title.is_empty() || title_has_private_marker(title) {
        NATIVE_SCAN_TITLE.to_owned()
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
