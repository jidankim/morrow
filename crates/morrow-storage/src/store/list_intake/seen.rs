use crate::sqlite_cli::sql_text;
use crate::types::ListIntakeExtractionDraft;
use crate::StorageError;

use crate::store::Store;

impl Store {
    pub(super) fn list_intake_table_count(&self, table: &str) -> Result<usize, StorageError> {
        let count = self
            .sqlite
            .query_scalar_i64(&format!("SELECT COUNT(*) FROM {table};"))?;
        usize::try_from(count).map_err(|err| StorageError::InvalidInput {
            field: "list_intake_table_count",
            reason: err.to_string(),
        })
    }

    pub(super) fn list_intake_extraction_seen(
        &self,
        draft: &ListIntakeExtractionDraft,
    ) -> Result<bool, StorageError> {
        let identity_filter = format!(
            "profile_id = {profile_id}
             AND profile_version = {profile_version}
             AND examples_hash = {examples_hash}
             AND message_guid = {message_guid}",
            profile_id = sql_text(&draft.profile_id)?,
            profile_version = sql_text(&draft.profile_version)?,
            examples_hash = sql_text(&draft.examples_hash)?,
            message_guid = sql_text(&draft.message_guid)?,
        );
        let count = self.sqlite.query_scalar_i64(&format!(
            "SELECT
                (SELECT COUNT(*) FROM list_intake_entries WHERE {identity_filter})
              + (SELECT COUNT(*) FROM list_intake_proposals WHERE {identity_filter});"
        ))?;
        Ok(count > 0)
    }
}
