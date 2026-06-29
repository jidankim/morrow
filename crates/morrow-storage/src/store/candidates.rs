use crate::ids::CandidateId;
use crate::sqlite_cli::sql_text;
use crate::types::CandidateState;
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
}
