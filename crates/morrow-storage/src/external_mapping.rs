use crate::sqlite_cli::{row_value, sql_text};
use crate::validation::{validate_mapping, validate_text};
use crate::{CandidateId, ExternalObjectMapping, ExternalSource, StorageError, Store};

impl Store {
    pub fn candidate_external_mapping(
        &self,
        candidate_id: &CandidateId,
        source: ExternalSource,
        mapped_at: i64,
    ) -> Result<Option<ExternalObjectMapping>, StorageError> {
        let sql = format!(
            "SELECT external_object_id, external_source_id
             FROM candidates
             WHERE id = {id}
               AND external_object_id IS NOT NULL
               AND external_source_id IS NOT NULL;",
            id = sql_text(candidate_id.as_str())?,
        );
        let rows = self.sqlite.query_rows(&sql)?;
        let Some(row) = rows.first() else {
            return Ok(None);
        };
        Ok(Some(ExternalObjectMapping {
            candidate_id: candidate_id.clone(),
            source,
            external_object_id: row_value(row, 0, "candidate.external_object_id")?.to_owned(),
            external_source_id: row_value(row, 1, "candidate.external_source_id")?.to_owned(),
            mapped_at,
        }))
    }

    pub fn record_candidate_external_receipt(
        &self,
        mapping: &ExternalObjectMapping,
    ) -> Result<(), StorageError> {
        validate_mapping(mapping)?;
        let sql = format!(
            "UPDATE candidates
             SET external_object_id = {object_id},
                 external_source_id = {source_id},
                 current_reason = 'external_proposal_receipt_recorded',
                 updated_at = {mapped_at}
             WHERE id = {id};",
            id = sql_text(mapping.candidate_id.as_str())?,
            object_id = sql_text(&mapping.external_object_id)?,
            source_id = sql_text(&mapping.external_source_id)?,
            mapped_at = mapping.mapped_at,
        );
        self.sqlite.execute(&sql)
    }

    pub fn record_external_replay_recovery_pending(
        &self,
        candidate_id: &CandidateId,
        reason: &str,
        observed_at: i64,
    ) -> Result<(), StorageError> {
        validate_text("reason", reason, 240)?;
        let state = self.candidate_state(candidate_id)?;
        let sql = format!(
            "BEGIN;
             UPDATE candidates SET current_reason = {reason}, updated_at = {observed_at}
             WHERE id = {id};
             INSERT INTO audit_log (candidate_id, from_state, to_state, reason, created_at)
             VALUES ({id}, {state}, {state}, {reason}, {observed_at});
             COMMIT;",
            id = sql_text(candidate_id.as_str())?,
            state = sql_text(state.as_str())?,
            reason = sql_text(reason)?,
        );
        self.sqlite.execute(&sql)
    }
}
