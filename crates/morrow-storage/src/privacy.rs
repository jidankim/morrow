use crate::sqlite_cli::Sqlite;
use crate::types::PrivacySummary;
use crate::StorageError;

pub(crate) fn summarize_privacy(
    sqlite: &Sqlite,
    table_count: usize,
) -> Result<PrivacySummary, StorageError> {
    let columns = sqlite.query_first_column(
        "SELECT name FROM pragma_table_info('evidence')
         UNION ALL SELECT name FROM pragma_table_info('quiet_logs')
         UNION ALL SELECT name FROM pragma_table_info('feedback_events')
         UNION ALL SELECT name FROM pragma_table_info('labels')
         UNION ALL SELECT name FROM pragma_table_info('feature_snapshots')
         UNION ALL SELECT name FROM pragma_table_info('eval_runs')
         UNION ALL SELECT name FROM pragma_table_info('eval_results')
         UNION ALL SELECT name FROM pragma_table_info('provider_route_outcomes')
         UNION ALL SELECT name FROM pragma_table_info('list_intake_entries')
         UNION ALL SELECT name FROM pragma_table_info('list_intake_proposals')
         UNION ALL SELECT name FROM pragma_table_info('list_intake_proposal_items')
         UNION ALL SELECT name FROM pragma_table_info('list_intake_sender_labels');",
    )?;
    let full_message_body_columns = columns
        .iter()
        .filter(|name| {
            let lowered = name.to_ascii_lowercase();
            lowered.contains("full_message")
                || lowered == "body"
                || lowered == "prompt"
                || lowered == "response"
                || lowered.ends_with("_prompt")
                || lowered.ends_with("_response")
                || lowered.contains("prompt_text")
                || lowered.contains("response_text")
        })
        .count();
    let max_excerpt_len = usize::try_from(sqlite.query_scalar_i64(
        "SELECT COALESCE(MAX(LENGTH(excerpt)), 0) FROM (
           SELECT excerpt FROM evidence
           UNION ALL
           SELECT excerpt FROM quiet_logs
           UNION ALL
           SELECT excerpt FROM feature_snapshots WHERE excerpt IS NOT NULL
           UNION ALL
           SELECT candidate_evidence_excerpt FROM provider_route_outcomes
           WHERE candidate_evidence_excerpt IS NOT NULL
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
