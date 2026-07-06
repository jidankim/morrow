use crate::sqlite_cli::{row_value, sql_text};
use crate::types::{
    CalendarProposalPayload, CandidateKind, CandidateState, ExternalSource,
    ReminderProposalPayload, PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE,
};
use crate::{CandidateId, StorageError, Store};

const SELECTED_MESSAGES_SOURCE_ID: &str = "morrow-selected-messages";
const SELECTED_REMINDERS_SOURCE_ID: &str = "morrow-selected-reminders";
const NATIVE_SCAN_REMINDER_TITLE: &str = "Messages reminder candidate";

impl Store {
    pub fn calendar_proposal_payloads(
        &self,
        candidate_ids: &[CandidateId],
    ) -> Result<Vec<CalendarProposalPayload>, StorageError> {
        let mut payloads = Vec::new();
        for candidate_id in candidate_ids {
            if let Some(payload) = self.calendar_proposal_payload(candidate_id)? {
                payloads.push(payload);
            }
        }
        Ok(payloads)
    }

    fn calendar_proposal_payload(
        &self,
        candidate_id: &CandidateId,
    ) -> Result<Option<CalendarProposalPayload>, StorageError> {
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
            source = sql_text(ExternalSource::Calendar.as_str())?,
            state = sql_text(CandidateState::CreatingExternal.as_str())?,
            kind = sql_text(CandidateKind::CalendarEvent.as_str())?,
        );
        let rows = self.sqlite.query_rows(&sql)?;
        let Some(row) = rows.first() else {
            return Ok(None);
        };
        let normalized_time = row_value(row, 2, "calendar_payload.normalized_time")?;
        if !is_normalized_calendar_time(normalized_time) {
            return Ok(None);
        }
        let title = payload_title(
            row_value(row, 3, "calendar_payload.title")?,
            PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE,
        );
        Ok(Some(CalendarProposalPayload {
            candidate_id: CandidateId::from_storage(row_value(row, 0, "calendar_payload.id")?)?,
            kind: CandidateKind::parse(row_value(row, 1, "calendar_payload.kind")?)?,
            normalized_time: normalized_time.to_owned(),
            title,
            source_id: SELECTED_MESSAGES_SOURCE_ID.to_owned(),
        }))
    }

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
        let normalized_time = row_value(row, 2, "reminder_payload.normalized_time")?;
        if !is_normalized_calendar_time(normalized_time) {
            return Ok(None);
        }
        let title = payload_title(
            row_value(row, 3, "reminder_payload.title")?,
            NATIVE_SCAN_REMINDER_TITLE,
        );
        Ok(Some(ReminderProposalPayload {
            candidate_id: CandidateId::from_storage(row_value(row, 0, "reminder_payload.id")?)?,
            kind: CandidateKind::parse(row_value(row, 1, "reminder_payload.kind")?)?,
            normalized_time: normalized_time.to_owned(),
            title,
            source_id: SELECTED_REMINDERS_SOURCE_ID.to_owned(),
        }))
    }
}

pub fn privacy_safe_native_scan_title(raw: &str, fallback: &str) -> String {
    let title = raw.trim();
    if title.is_empty() || title_has_private_marker(title) {
        fallback.to_owned()
    } else {
        title.to_owned()
    }
}

fn payload_title(raw: &str, fallback: &str) -> String {
    privacy_safe_native_scan_title(raw, fallback)
}

fn title_has_private_marker(title: &str) -> bool {
    let lowered = title.to_ascii_lowercase();
    title.contains('@')
        || title.contains('+')
        || lowered.contains("private")
        || lowered.contains("raw-")
        || title_looks_like_provider_contract(title)
        || has_phone_like_digit_sequence(title, 7)
}

fn title_looks_like_provider_contract(title: &str) -> bool {
    let lowered = title.to_ascii_lowercase();
    title.starts_with('{')
        && title.ends_with('}')
        && lowered.contains("\"kind\"")
        && lowered.contains("\"title\"")
        && (lowered.contains("\"calendar_event\"") || lowered.contains("\"task_reminder\""))
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

fn is_normalized_calendar_time(value: &str) -> bool {
    let bytes = value.as_bytes();
    is_utc_normalized_calendar_time(bytes) || is_timezone_normalized_calendar_time(bytes)
}

fn is_utc_normalized_calendar_time(bytes: &[u8]) -> bool {
    bytes.len() == 20 && calendar_datetime_fields(bytes) && bytes[19] == b'Z'
}

fn is_timezone_normalized_calendar_time(bytes: &[u8]) -> bool {
    bytes.len() > 21
        && calendar_datetime_fields(bytes)
        && bytes[19] == b'['
        && bytes[bytes.len() - 1] == b']'
        && timezone_name(&bytes[20..bytes.len() - 1])
}

fn calendar_datetime_fields(bytes: &[u8]) -> bool {
    bytes.len() >= 19
        && digits(&bytes[0..4])
        && bytes[4] == b'-'
        && digits(&bytes[5..7])
        && bytes[7] == b'-'
        && digits(&bytes[8..10])
        && bytes[10] == b'T'
        && digits(&bytes[11..13])
        && bytes[13] == b':'
        && digits(&bytes[14..16])
        && bytes[16] == b':'
        && digits(&bytes[17..19])
}

fn digits(bytes: &[u8]) -> bool {
    bytes.iter().all(u8::is_ascii_digit)
}

fn timezone_name(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'_' | b'-'))
}
