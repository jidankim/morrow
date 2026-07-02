use crate::ids::CandidateId;
use crate::sqlite_cli::{row_value, sql_text};
use crate::types::{
    CandidateFeedbackContext, CandidateKind, CandidateLifecycleReadback, CandidateState,
};
use crate::StorageError;

use super::Store;

const CANDIDATE_SUPERSEDED_REASON: &str = "candidate_superseded";
const CALENDAR_KIND_FAMILY: [CandidateKind; 4] = [
    CandidateKind::CalendarEvent,
    CandidateKind::EventUpdate,
    CandidateKind::EventReschedule,
    CandidateKind::EventCancellation,
];
const REMINDER_KIND_FAMILY: [CandidateKind; 4] = [
    CandidateKind::TaskReminder,
    CandidateKind::ReminderUpdate,
    CandidateKind::ReminderReschedule,
    CandidateKind::ReminderCancellation,
];

impl Store {
    pub fn candidate_state(
        &self,
        candidate_id: &CandidateId,
    ) -> Result<CandidateState, StorageError> {
        let sql = format!(
            "SELECT state FROM candidates WHERE id = {};",
            sql_text(candidate_id.as_str())?
        );
        let rows = self.sqlite.query_first_column(&sql)?;
        let raw = rows.first().ok_or_else(|| StorageError::CandidateMissing {
            id: candidate_id.to_string(),
        })?;
        CandidateState::parse(raw)
    }

    pub fn candidate_feedback_context(
        &self,
        candidate_id: &CandidateId,
    ) -> Result<CandidateFeedbackContext, StorageError> {
        let sql = format!(
            "SELECT chat_guid, anchor_message_guid FROM candidates WHERE id = {};",
            sql_text(candidate_id.as_str())?
        );
        let rows = self.sqlite.query_rows(&sql)?;
        let row = rows.first().ok_or_else(|| StorageError::CandidateMissing {
            id: candidate_id.to_string(),
        })?;
        Ok(CandidateFeedbackContext {
            candidate_id: candidate_id.clone(),
            chat_guid: row_value(row, 0, "candidate.chat_guid")?.to_owned(),
            anchor_message_guid: row_value(row, 1, "candidate.anchor_message_guid")?.to_owned(),
        })
    }

    pub fn candidate_lifecycle_readback(
        &self,
        candidate_id: &CandidateId,
    ) -> Result<CandidateLifecycleReadback, StorageError> {
        let sql = format!(
            "SELECT id, kind, state, current_reason FROM candidates WHERE id = {};",
            sql_text(candidate_id.as_str())?
        );
        let rows = self.sqlite.query_rows(&sql)?;
        let row = rows.first().ok_or_else(|| StorageError::CandidateMissing {
            id: candidate_id.to_string(),
        })?;
        Ok(CandidateLifecycleReadback {
            candidate_id: CandidateId::from_storage(row_value(row, 0, "candidate.id")?)?,
            kind: CandidateKind::parse(row_value(row, 1, "candidate.kind")?)?,
            state: CandidateState::parse(row_value(row, 2, "candidate.state")?)?,
            current_reason: row_value(row, 3, "candidate.current_reason")?.to_owned(),
        })
    }

    pub fn supersede_prior_candidates_by_anchor(
        &self,
        candidate_id: &CandidateId,
        observed_at: i64,
    ) -> Result<Vec<CandidateId>, StorageError> {
        let anchor = self.candidate_anchor(candidate_id)?;
        let quoted_kinds = kind_family(anchor.kind)
            .iter()
            .map(|kind| sql_text(kind.as_str()))
            .collect::<Result<Vec<_>, _>>()?;
        let sql = format!(
            "SELECT id
             FROM candidates
             WHERE chat_guid = {chat_guid}
               AND anchor_message_guid = {anchor_message_guid}
               AND kind IN ({kinds})
               AND id != {id}
               AND state IN ('queued', 'creating_external', 'visible')
               AND (
                   created_at < {created_at}
                   OR (created_at = {created_at} AND rowid < {rowid})
               )
             ORDER BY created_at, rowid, id;",
            chat_guid = sql_text(&anchor.chat_guid)?,
            anchor_message_guid = sql_text(&anchor.anchor_message_guid)?,
            kinds = quoted_kinds.join(", "),
            id = sql_text(candidate_id.as_str())?,
            created_at = anchor.created_at,
            rowid = anchor.rowid,
        );
        let prior_ids = self
            .sqlite
            .query_first_column(&sql)?
            .into_iter()
            .map(|raw| CandidateId::from_storage(&raw))
            .collect::<Result<Vec<_>, _>>()?;
        for prior_id in &prior_ids {
            self.transition_candidate(
                prior_id,
                CandidateState::Suppressed,
                CANDIDATE_SUPERSEDED_REASON,
                observed_at,
            )?;
        }
        Ok(prior_ids)
    }

    fn candidate_anchor(
        &self,
        candidate_id: &CandidateId,
    ) -> Result<CandidateAnchor, StorageError> {
        let sql = format!(
            "SELECT kind, chat_guid, anchor_message_guid, created_at, rowid FROM candidates WHERE id = {};",
            sql_text(candidate_id.as_str())?
        );
        let rows = self.sqlite.query_rows(&sql)?;
        let row = rows.first().ok_or_else(|| StorageError::CandidateMissing {
            id: candidate_id.to_string(),
        })?;
        Ok(CandidateAnchor {
            kind: CandidateKind::parse(row_value(row, 0, "candidate.kind")?)?,
            chat_guid: row_value(row, 1, "candidate.chat_guid")?.to_owned(),
            anchor_message_guid: row_value(row, 2, "candidate.anchor_message_guid")?.to_owned(),
            created_at: row_value(row, 3, "candidate.created_at")?
                .parse::<i64>()
                .map_err(|_| StorageError::InvalidInput {
                    field: "candidate.created_at",
                    reason: "must be an integer".to_owned(),
                })?,
            rowid: row_value(row, 4, "candidate.rowid")?
                .parse::<i64>()
                .map_err(|_| StorageError::InvalidInput {
                    field: "candidate.rowid",
                    reason: "must be an integer".to_owned(),
                })?,
        })
    }
}

struct CandidateAnchor {
    kind: CandidateKind,
    chat_guid: String,
    anchor_message_guid: String,
    created_at: i64,
    rowid: i64,
}

const fn kind_family(kind: CandidateKind) -> &'static [CandidateKind; 4] {
    match kind {
        CandidateKind::CalendarEvent
        | CandidateKind::EventUpdate
        | CandidateKind::EventReschedule
        | CandidateKind::EventCancellation => &CALENDAR_KIND_FAMILY,
        CandidateKind::TaskReminder
        | CandidateKind::ReminderUpdate
        | CandidateKind::ReminderReschedule
        | CandidateKind::ReminderCancellation => &REMINDER_KIND_FAMILY,
    }
}
