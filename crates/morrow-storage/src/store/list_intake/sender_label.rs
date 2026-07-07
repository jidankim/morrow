use crate::sqlite_cli::sql_text;
use crate::types::ListIntakeExtractionDraft;
use crate::StorageError;

use super::row::{sender_key_bucket, UNKNOWN_SENDER_BUCKET};

pub(super) fn insert_sql(draft: &ListIntakeExtractionDraft) -> Result<String, StorageError> {
    let bucket = sender_key_bucket(draft.sender_key.as_deref());
    Ok(format!(
        "INSERT OR IGNORE INTO list_intake_sender_labels
         (profile_id, window_start_unix_seconds, chat_key, sender_key, sender_key_bucket,
          sender_label, created_at, updated_at)
         VALUES ({profile_id}, {window_start}, {chat_key}, {sender_key}, {bucket},
                 {label}, {created_at}, {created_at});\n",
        profile_id = sql_text(&draft.profile_id)?,
        window_start = draft.window.window_start_unix_seconds,
        chat_key = sql_text(&draft.chat_key)?,
        sender_key = sql_optional_text(draft.sender_key.as_deref())?,
        bucket = sql_text(&bucket)?,
        label = insert_value(draft)?,
        created_at = draft.observed_at,
    ))
}

pub(super) fn id_subquery(draft: &ListIntakeExtractionDraft) -> Result<String, StorageError> {
    Ok(format!(
        "(SELECT id FROM list_intake_sender_labels
          WHERE profile_id = {profile_id}
            AND window_start_unix_seconds = {window_start}
            AND chat_key = {chat_key}
            AND sender_key_bucket = {bucket})",
        profile_id = sql_text(&draft.profile_id)?,
        window_start = draft.window.window_start_unix_seconds,
        chat_key = sql_text(&draft.chat_key)?,
        bucket = sql_text(&sender_key_bucket(draft.sender_key.as_deref()))?,
    ))
}

pub(super) fn value_subquery(draft: &ListIntakeExtractionDraft) -> Result<String, StorageError> {
    Ok(format!(
        "(SELECT sender_label FROM list_intake_sender_labels
          WHERE profile_id = {profile_id}
            AND window_start_unix_seconds = {window_start}
            AND chat_key = {chat_key}
            AND sender_key_bucket = {bucket})",
        profile_id = sql_text(&draft.profile_id)?,
        window_start = draft.window.window_start_unix_seconds,
        chat_key = sql_text(&draft.chat_key)?,
        bucket = sql_text(&sender_key_bucket(draft.sender_key.as_deref()))?,
    ))
}

fn insert_value(draft: &ListIntakeExtractionDraft) -> Result<String, StorageError> {
    if draft.sender_key.is_none() {
        return sql_text("Unknown sender");
    }
    let bucket = sender_key_bucket(draft.sender_key.as_deref());
    Ok(format!(
        "'Sender ' || MAX(
            (
                SELECT COUNT(*) + 1
                FROM list_intake_sender_labels
                WHERE profile_id = {profile_id}
                  AND window_start_unix_seconds = {window_start}
                  AND chat_key = {chat_key}
                  AND sender_key_bucket != {unknown_bucket}
                  AND (
                      created_at < {created_at}
                      OR (created_at = {created_at} AND sender_key_bucket < {bucket})
                  )
            ),
            (
                SELECT COUNT(*) + 1
                FROM list_intake_sender_labels
                WHERE profile_id = {profile_id}
                  AND window_start_unix_seconds = {window_start}
                  AND chat_key = {chat_key}
                  AND sender_key_bucket != {unknown_bucket}
            )
        )",
        profile_id = sql_text(&draft.profile_id)?,
        window_start = draft.window.window_start_unix_seconds,
        chat_key = sql_text(&draft.chat_key)?,
        bucket = sql_text(&bucket)?,
        unknown_bucket = sql_text(UNKNOWN_SENDER_BUCKET)?,
        created_at = draft.observed_at,
    ))
}

fn sql_optional_text(value: Option<&str>) -> Result<String, StorageError> {
    value.map_or_else(|| Ok("NULL".to_owned()), sql_text)
}
