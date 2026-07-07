use crate::sqlite_cli::{row_value, sql_text};
use crate::types::{
    ListIntakeAggregateQuery, ListIntakeAggregateRow, ListIntakeConfidenceTier,
    ListIntakeExtractionDraft, ListIntakeItemDraft,
};
use crate::StorageError;

use super::row::{parse_i64, parse_optional_text, storage_id};
use super::sender_label;

pub(super) fn entry_insert_sql(
    draft: &ListIntakeExtractionDraft,
    item: &ListIntakeItemDraft,
    item_index: usize,
    idempotency_key: &str,
    source_proposal_id: Option<&str>,
) -> Result<String, StorageError> {
    let entry_id = storage_id("list_intake_entry", idempotency_key);
    Ok(format!(
        "{}INSERT OR IGNORE INTO list_intake_entries
         (entry_id, idempotency_key, profile_id, profile_version, examples_hash,
          source_proposal_id, message_guid, evidence_pointer, chat_key, sender_key,
          sender_label_id, sender_label, window_local_date, window_timezone,
          window_start_unix_seconds, source_item_index, quantity, item_name, unit,
          category_id, confidence_tier, confidence_millis, created_at, updated_at)
         VALUES
         ({entry_id}, {idempotency_key}, {profile_id}, {profile_version}, {examples_hash},
          {source_proposal_id}, {message_guid}, {evidence_pointer}, {chat_key}, {sender_key},
          {sender_label_id}, {sender_label}, {window_local_date}, {window_timezone},
          {window_start}, {item_index}, {quantity}, {item_name}, {unit}, {category_id},
          {confidence_tier}, {confidence_millis}, {created_at}, {created_at});\n",
        sender_label::insert_sql(draft)?,
        entry_id = sql_text(&entry_id)?,
        idempotency_key = sql_text(idempotency_key)?,
        profile_id = sql_text(&draft.profile_id)?,
        profile_version = sql_text(&draft.profile_version)?,
        examples_hash = sql_text(&draft.examples_hash)?,
        source_proposal_id = sql_optional_text(source_proposal_id)?,
        message_guid = sql_text(&draft.message_guid)?,
        evidence_pointer = sql_text(&draft.evidence_pointer)?,
        chat_key = sql_text(&draft.chat_key)?,
        sender_key = sql_optional_text(draft.sender_key.as_deref())?,
        sender_label_id = sender_label::id_subquery(draft)?,
        sender_label = sender_label::value_subquery(draft)?,
        window_local_date = sql_text(&draft.window.window_local_date)?,
        window_timezone = sql_text(&draft.window.window_timezone)?,
        window_start = draft.window.window_start_unix_seconds,
        quantity = item.quantity,
        item_name = sql_text(&item.item_name)?,
        unit = sql_optional_text(item.unit.as_deref())?,
        category_id = sql_text(&item.category_id)?,
        confidence_tier = sql_text(draft.confidence_tier.as_str())?,
        confidence_millis = draft.confidence_millis,
        created_at = draft.observed_at,
    ))
}

pub(super) fn proposal_insert_sql(
    draft: &ListIntakeExtractionDraft,
    idempotency_key: &str,
) -> Result<String, StorageError> {
    let proposal_id = storage_id("list_intake_proposal", idempotency_key);
    let item = draft
        .items
        .first()
        .ok_or_else(|| StorageError::InvalidInput {
            field: "items",
            reason: "must contain at least one item".to_owned(),
        })?;
    Ok(format!(
        "{}INSERT OR IGNORE INTO list_intake_proposals
         (proposal_id, idempotency_key, status, profile_id, profile_version, examples_hash,
          message_guid, evidence_pointer, chat_key, sender_key, sender_label_id, sender_label,
          window_local_date, window_timezone, window_start_unix_seconds, source_item_index,
          quantity, item_name, unit, category_id, confidence_tier, confidence_millis,
          created_at, updated_at)
         VALUES
        ({proposal_id}, {idempotency_key}, 'pending', {profile_id}, {profile_version},
         {examples_hash}, {message_guid}, {evidence_pointer}, {chat_key}, {sender_key},
         {sender_label_id}, {sender_label}, {window_local_date}, {window_timezone},
          {window_start}, 0, {quantity}, {item_name}, {unit}, {category_id},
          {confidence_tier}, {confidence_millis}, {created_at}, {created_at});\n",
        sender_label::insert_sql(draft)?,
        proposal_id = sql_text(&proposal_id)?,
        idempotency_key = sql_text(idempotency_key)?,
        profile_id = sql_text(&draft.profile_id)?,
        profile_version = sql_text(&draft.profile_version)?,
        examples_hash = sql_text(&draft.examples_hash)?,
        message_guid = sql_text(&draft.message_guid)?,
        evidence_pointer = sql_text(&draft.evidence_pointer)?,
        chat_key = sql_text(&draft.chat_key)?,
        sender_key = sql_optional_text(draft.sender_key.as_deref())?,
        sender_label_id = sender_label::id_subquery(draft)?,
        sender_label = sender_label::value_subquery(draft)?,
        window_local_date = sql_text(&draft.window.window_local_date)?,
        window_timezone = sql_text(&draft.window.window_timezone)?,
        window_start = draft.window.window_start_unix_seconds,
        quantity = item.quantity,
        item_name = sql_text(&item.item_name)?,
        unit = sql_optional_text(item.unit.as_deref())?,
        category_id = sql_text(&item.category_id)?,
        confidence_tier = sql_text(draft.confidence_tier.as_str())?,
        confidence_millis = draft.confidence_millis,
        created_at = draft.observed_at,
    ))
}

pub(super) fn proposal_item_insert_sql(
    proposal_id: &str,
    item: &ListIntakeItemDraft,
    item_index: usize,
    idempotency_key: &str,
    created_at: i64,
) -> Result<String, StorageError> {
    let item_id = storage_id("list_intake_proposal_item", idempotency_key);
    Ok(format!(
        "INSERT OR IGNORE INTO list_intake_proposal_items
         (item_id, idempotency_key, proposal_id, source_item_index, quantity, item_name,
          unit, category_id, created_at, updated_at)
         VALUES
         ({item_id}, {idempotency_key}, {proposal_id}, {item_index}, {quantity},
          {item_name}, {unit}, {category_id}, {created_at}, {created_at});\n",
        item_id = sql_text(&item_id)?,
        idempotency_key = sql_text(idempotency_key)?,
        proposal_id = sql_text(proposal_id)?,
        quantity = item.quantity,
        item_name = sql_text(&item.item_name)?,
        unit = sql_optional_text(item.unit.as_deref())?,
        category_id = sql_text(&item.category_id)?,
    ))
}

pub(super) fn aggregate_sql(query: &ListIntakeAggregateQuery) -> Result<String, StorageError> {
    let mut filters = Vec::new();
    push_filter(&mut filters, "profile_id", query.profile_id.as_deref())?;
    push_filter(
        &mut filters,
        "window_local_date",
        query.window_local_date.as_deref(),
    )?;
    push_filter(&mut filters, "chat_key", query.chat_key.as_deref())?;
    push_filter(&mut filters, "sender_label", query.sender_label.as_deref())?;
    push_filter(&mut filters, "category_id", query.category_id.as_deref())?;
    let where_clause = if filters.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", filters.join(" AND "))
    };
    Ok(format!(
        "SELECT profile_id, profile_version, examples_hash, window_local_date,
                window_timezone, window_start_unix_seconds, chat_key, MIN(sender_label), category_id,
                item_name, unit, SUM(quantity), COUNT(*),
                SUM(CASE WHEN source_proposal_id IS NULL THEN 0 ELSE 1 END)
         FROM list_intake_entries
         {where_clause}
         GROUP BY profile_id, profile_version, examples_hash, window_local_date,
                  window_timezone, window_start_unix_seconds, chat_key,
                  COALESCE(sender_key, '__unknown__'),
                  category_id, item_name, unit
         ORDER BY MIN(sender_label), category_id, item_name, unit;"
    ))
}

pub(super) fn aggregate_row(row: &[String]) -> Result<ListIntakeAggregateRow, StorageError> {
    Ok(ListIntakeAggregateRow {
        profile_id: row_value(row, 0, "profile_id")?.to_owned(),
        profile_version: row_value(row, 1, "profile_version")?.to_owned(),
        examples_hash: row_value(row, 2, "examples_hash")?.to_owned(),
        window_local_date: row_value(row, 3, "window_local_date")?.to_owned(),
        window_timezone: row_value(row, 4, "window_timezone")?.to_owned(),
        window_start_unix_seconds: parse_i64(
            row_value(row, 5, "window_start_unix_seconds")?,
            "window_start_unix_seconds",
        )?,
        chat_key: row_value(row, 6, "chat_key")?.to_owned(),
        sender_label: row_value(row, 7, "sender_label")?.to_owned(),
        category_id: row_value(row, 8, "category_id")?.to_owned(),
        item_name: row_value(row, 9, "item_name")?.to_owned(),
        unit: parse_optional_text(row_value(row, 10, "unit")?),
        total_quantity: parse_i64(row_value(row, 11, "total_quantity")?, "total_quantity")?,
        entry_count: parse_i64(row_value(row, 12, "entry_count")?, "entry_count")?,
        source_proposal_count: parse_i64(
            row_value(row, 13, "source_proposal_count")?,
            "source_proposal_count",
        )?,
    })
}

pub(super) fn proposal_source_draft(
    row: &[String],
    observed_at: i64,
) -> Result<ListIntakeExtractionDraft, StorageError> {
    Ok(ListIntakeExtractionDraft {
        profile_id: row_value(row, 1, "profile_id")?.to_owned(),
        profile_version: row_value(row, 2, "profile_version")?.to_owned(),
        examples_hash: row_value(row, 3, "examples_hash")?.to_owned(),
        message_guid: row_value(row, 4, "message_guid")?.to_owned(),
        evidence_pointer: row_value(row, 5, "evidence_pointer")?.to_owned(),
        chat_key: row_value(row, 6, "chat_key")?.to_owned(),
        sender_key: parse_optional_text(row_value(row, 7, "sender_key")?),
        window: crate::types::ListIntakeLocalDayWindow {
            window_local_date: row_value(row, 8, "window_local_date")?.to_owned(),
            window_timezone: row_value(row, 9, "window_timezone")?.to_owned(),
            window_start_unix_seconds: parse_i64(
                row_value(row, 10, "window_start_unix_seconds")?,
                "window_start_unix_seconds",
            )?,
        },
        confidence_tier: ListIntakeConfidenceTier::parse(row_value(row, 12, "confidence_tier")?)?,
        confidence_millis: parse_i64(
            row_value(row, 11, "confidence_millis")?,
            "confidence_millis",
        )?,
        items: Vec::new(),
        observed_at,
    })
}

pub(super) fn count_delta(before: usize, after: usize) -> Result<usize, StorageError> {
    after
        .checked_sub(before)
        .ok_or_else(|| StorageError::Sqlite {
            message: "list intake count moved backwards".to_owned(),
        })
}

fn push_filter(
    filters: &mut Vec<String>,
    column: &'static str,
    value: Option<&str>,
) -> Result<(), StorageError> {
    if let Some(value) = value {
        filters.push(format!("{column} = {}", sql_text(value)?));
    }
    Ok(())
}

fn sql_optional_text(value: Option<&str>) -> Result<String, StorageError> {
    value.map_or_else(|| Ok("NULL".to_owned()), sql_text)
}
