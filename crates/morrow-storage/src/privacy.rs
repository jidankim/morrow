use crate::sqlite_cli::Sqlite;
use crate::types::PrivacySummary;
use crate::StorageError;

pub(crate) fn summarize_privacy(
    sqlite: &Sqlite,
    table_count: usize,
) -> Result<PrivacySummary, StorageError> {
    let columns = sqlite.query_first_column(
        "SELECT name FROM pragma_table_info('evidence')
         UNION ALL
         SELECT name FROM pragma_table_info('quiet_logs');",
    )?;
    let full_message_body_columns = columns
        .iter()
        .filter(|name| {
            let lowered = name.to_ascii_lowercase();
            lowered.contains("full_message")
                || lowered == "body"
                || lowered.contains("prompt")
                || lowered.contains("response")
        })
        .count();
    let max_excerpt_len = usize::try_from(sqlite.query_scalar_i64(
        "SELECT COALESCE(MAX(LENGTH(excerpt)), 0) FROM (
           SELECT excerpt FROM evidence
           UNION ALL
           SELECT excerpt FROM quiet_logs
         );",
    )?)
    .map_err(|err| StorageError::InvalidInput {
        field: "max_excerpt_len",
        reason: err.to_string(),
    })?;
    Ok(PrivacySummary {
        table_count,
        full_message_body_columns,
        max_excerpt_len,
    })
}
