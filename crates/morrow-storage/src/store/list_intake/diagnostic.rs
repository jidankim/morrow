use crate::sqlite_cli::{row_value, sql_text};
use crate::types::{ListIntakeProviderDiagnostic, ListIntakeProviderDiagnosticDraft};
use crate::StorageError;

use super::Store;

impl Store {
    pub fn record_list_intake_provider_diagnostic(
        &self,
        draft: ListIntakeProviderDiagnosticDraft,
    ) -> Result<(), StorageError> {
        validate_provider_diagnostic(&draft)?;
        let sql = format!(
            "INSERT OR IGNORE INTO list_intake_provider_diagnostics
             (diagnostic_key, profile_id, profile_version, examples_hash, message_hash, chat_key,
              provider_prompt_version, provider_schema_version, reason_code, retry_state, created_at)
             VALUES ({diagnostic_key}, {profile_id}, {profile_version}, {examples_hash},
                     {message_hash}, {chat_key}, {provider_prompt_version},
                     {provider_schema_version}, {reason_code}, {retry_state}, {created_at});",
            diagnostic_key = sql_text(&draft.diagnostic_key)?,
            profile_id = sql_text(&draft.profile_id)?,
            profile_version = sql_text(&draft.profile_version)?,
            examples_hash = sql_text(&draft.examples_hash)?,
            message_hash = sql_text(&draft.message_hash)?,
            chat_key = sql_text(&draft.chat_key)?,
            provider_prompt_version = sql_text(&draft.provider_prompt_version)?,
            provider_schema_version = sql_text(&draft.provider_schema_version)?,
            reason_code = sql_text(&draft.reason_code)?,
            retry_state = sql_text(&draft.retry_state)?,
            created_at = draft.created_at,
        );
        self.sqlite.execute(&sql)
    }

    pub fn list_intake_provider_diagnostics(
        &self,
        profile_id: &str,
    ) -> Result<Vec<ListIntakeProviderDiagnostic>, StorageError> {
        crate::validation::validate_text("profile_id", profile_id, 80)?;
        let sql = format!(
            "SELECT profile_id, message_hash, reason_code, retry_state
             FROM list_intake_provider_diagnostics
             WHERE profile_id = {}
             ORDER BY created_at, diagnostic_key;",
            sql_text(profile_id)?
        );
        self.sqlite
            .query_rows(&sql)?
            .into_iter()
            .map(|row| {
                Ok(ListIntakeProviderDiagnostic {
                    profile_id: row_value(&row, 0, "profile_id")?.to_owned(),
                    message_hash: row_value(&row, 1, "message_hash")?.to_owned(),
                    reason_code: row_value(&row, 2, "reason_code")?.to_owned(),
                    retry_state: row_value(&row, 3, "retry_state")?.to_owned(),
                })
            })
            .collect()
    }
}

fn validate_provider_diagnostic(
    draft: &ListIntakeProviderDiagnosticDraft,
) -> Result<(), StorageError> {
    crate::validation::validate_text("diagnostic_key", &draft.diagnostic_key, 160)?;
    crate::validation::validate_text("profile_id", &draft.profile_id, 80)?;
    crate::validation::validate_text("profile_version", &draft.profile_version, 40)?;
    crate::validation::validate_text("examples_hash", &draft.examples_hash, 160)?;
    crate::validation::validate_text("message_hash", &draft.message_hash, 80)?;
    crate::validation::validate_text("chat_key", &draft.chat_key, 80)?;
    crate::validation::validate_text(
        "provider_prompt_version",
        &draft.provider_prompt_version,
        80,
    )?;
    crate::validation::validate_text(
        "provider_schema_version",
        &draft.provider_schema_version,
        80,
    )?;
    crate::validation::validate_text("reason_code", &draft.reason_code, 80)?;
    crate::validation::validate_text("retry_state", &draft.retry_state, 40)?;
    if draft.retry_state != "retry_available" {
        return Err(StorageError::InvalidInput {
            field: "retry_state",
            reason: "must be retry_available".to_owned(),
        });
    }
    Ok(())
}
