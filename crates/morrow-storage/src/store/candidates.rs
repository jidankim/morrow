use crate::ids::CandidateId;
use crate::sqlite_cli::{row_value, sql_text};
use crate::types::{CandidateFeedbackContext, CandidateState};
use crate::StorageError;

use super::Store;

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
}
