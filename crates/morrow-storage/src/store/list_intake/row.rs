use crate::sqlite_cli::row_value;
use crate::types::{
    ListIntakeExtractionDraft, ListIntakeItemDraft, ListIntakeLocalDayWindow, ListIntakeProposal,
    ListIntakeProposalStatus, LIST_INTAKE_UNCATEGORIZED_CATEGORY_ID,
};
use crate::validation::validate_text;
use crate::StorageError;

pub(super) const UNKNOWN_SENDER_BUCKET: &str = "__unknown__";
const FNV_OFFSET: u32 = 2_166_136_261;
const FNV_PRIME: u32 = 16_777_619;

pub(super) fn validate_extraction(draft: &ListIntakeExtractionDraft) -> Result<(), StorageError> {
    validate_metadata_token("profile_id", &draft.profile_id, 80)?;
    validate_metadata_token("profile_version", &draft.profile_version, 120)?;
    validate_metadata_token("examples_hash", &draft.examples_hash, 160)?;
    validate_text("message_guid", &draft.message_guid, 240)?;
    validate_text("evidence_pointer", &draft.evidence_pointer, 240)?;
    validate_metadata_token("chat_key", &draft.chat_key, 160)?;
    validate_sender_key(draft.sender_key.as_deref())?;
    validate_window(&draft.window)?;
    validate_confidence(draft.confidence_millis)?;
    if draft.items.is_empty() {
        return Err(StorageError::InvalidInput {
            field: "items",
            reason: "must contain at least one item".to_owned(),
        });
    }
    for item in &draft.items {
        validate_item(item)?;
    }
    Ok(())
}

pub(super) fn validate_item(item: &ListIntakeItemDraft) -> Result<(), StorageError> {
    validate_item_bounds(item)?;
    validate_category_token(&item.category_id)
}

pub(super) fn validate_item_with_categories(
    item: &ListIntakeItemDraft,
    allowed_category_ids: &[String],
) -> Result<(), StorageError> {
    validate_item_bounds(item)?;
    validate_category_token(&item.category_id)?;
    validate_allowed_categories(allowed_category_ids)?;
    if item.category_id == LIST_INTAKE_UNCATEGORIZED_CATEGORY_ID
        || allowed_category_ids
            .iter()
            .any(|category_id| category_id == &item.category_id)
    {
        Ok(())
    } else {
        Err(StorageError::InvalidInput {
            field: "category_id",
            reason: "must be uncategorized or one configured category ID".to_owned(),
        })
    }
}

fn validate_item_bounds(item: &ListIntakeItemDraft) -> Result<(), StorageError> {
    validate_visible_text("item_name", &item.item_name, 1, 80)?;
    if let Some(unit) = &item.unit {
        validate_visible_text("unit", unit, 1, 24)?;
    }
    if (1..=999).contains(&item.quantity) {
        Ok(())
    } else {
        Err(StorageError::InvalidInput {
            field: "quantity",
            reason: "must be between 1 and 999".to_owned(),
        })
    }
}

fn validate_category_token(category_id: &str) -> Result<(), StorageError> {
    validate_metadata_token("category_id", category_id, 80)
}

fn validate_allowed_categories(allowed_category_ids: &[String]) -> Result<(), StorageError> {
    for category_id in allowed_category_ids {
        validate_category_token(category_id)?;
    }
    Ok(())
}

pub(super) fn sender_key_bucket(sender_key: Option<&str>) -> String {
    sender_key.map_or_else(|| UNKNOWN_SENDER_BUCKET.to_owned(), stable_hex)
}

pub(super) fn storage_id(prefix: &str, material: &str) -> String {
    format!("{prefix}_{}", stable_hex(material))
}

pub(super) fn extraction_item_key(draft: &ListIntakeExtractionDraft, item_index: usize) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        draft.profile_id,
        draft.profile_version,
        draft.examples_hash,
        draft.message_guid,
        item_index
    )
}

pub(super) fn extraction_proposal_key(draft: &ListIntakeExtractionDraft) -> String {
    format!(
        "{}|{}|{}|{}",
        draft.profile_id, draft.profile_version, draft.examples_hash, draft.message_guid
    )
}

pub(super) fn proposal_entry_key(proposal_id: &str, item_index: usize) -> String {
    format!("approved_proposal|{proposal_id}|{item_index}")
}

pub(super) fn parse_i64(raw: &str, field: &'static str) -> Result<i64, StorageError> {
    raw.parse::<i64>()
        .map_err(|err| StorageError::InvalidInput {
            field,
            reason: err.to_string(),
        })
}

pub(super) fn parse_optional_text(raw: &str) -> Option<String> {
    if raw.is_empty() {
        None
    } else {
        Some(raw.to_owned())
    }
}

pub(super) fn parse_proposal_row(
    row: &[String],
    items: Vec<ListIntakeItemDraft>,
) -> Result<ListIntakeProposal, StorageError> {
    Ok(ListIntakeProposal {
        proposal_id: row_value(row, 0, "proposal_id")?.to_owned(),
        status: ListIntakeProposalStatus::parse(row_value(row, 1, "status")?)?,
        profile_id: row_value(row, 2, "profile_id")?.to_owned(),
        message_guid: row_value(row, 3, "message_guid")?.to_owned(),
        sender_label: row_value(row, 4, "sender_label")?.to_owned(),
        items,
    })
}

pub(super) fn parse_item_row(row: &[String]) -> Result<ListIntakeItemDraft, StorageError> {
    Ok(ListIntakeItemDraft {
        item_name: row_value(row, 2, "item_name")?.to_owned(),
        quantity: parse_i64(row_value(row, 1, "quantity")?, "quantity")?,
        unit: parse_optional_text(row_value(row, 3, "unit")?),
        category_id: row_value(row, 4, "category_id")?.to_owned(),
    })
}

fn validate_window(window: &ListIntakeLocalDayWindow) -> Result<(), StorageError> {
    validate_text("window_local_date", &window.window_local_date, 24)?;
    validate_text("window_timezone", &window.window_timezone, 80)
}

fn validate_confidence(confidence_millis: i64) -> Result<(), StorageError> {
    if (0..=1000).contains(&confidence_millis) {
        Ok(())
    } else {
        Err(StorageError::InvalidInput {
            field: "confidence_millis",
            reason: "must be between 0 and 1000".to_owned(),
        })
    }
}

fn validate_sender_key(value: Option<&str>) -> Result<(), StorageError> {
    let Some(sender_key) = value else {
        return Ok(());
    };
    validate_metadata_token("sender_key", sender_key, 160)?;
    let lowered = sender_key.to_ascii_lowercase();
    let digit_count = sender_key.chars().filter(char::is_ascii_digit).count();
    let internal_sender_key = sender_key.starts_with("senderKey-");
    if lowered.contains('@')
        || lowered.contains("from:")
        || lowered.contains("to:")
        || (digit_count >= 7 && !internal_sender_key)
    {
        Err(StorageError::PrivacyViolation {
            reason: "sender_key must be an internal non-raw token".to_owned(),
        })
    } else {
        Ok(())
    }
}

fn validate_metadata_token(
    field: &'static str,
    value: &str,
    max_len: usize,
) -> Result<(), StorageError> {
    validate_text(field, value, max_len)?;
    if value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        Ok(())
    } else {
        Err(StorageError::InvalidInput {
            field,
            reason: "must be a bounded metadata token".to_owned(),
        })
    }
}

fn validate_visible_text(
    field: &'static str,
    value: &str,
    min_chars: usize,
    max_chars: usize,
) -> Result<(), StorageError> {
    validate_text(field, value, max_chars)?;
    let visible_chars = value.trim().chars().count();
    if (min_chars..=max_chars).contains(&visible_chars) {
        Ok(())
    } else {
        Err(StorageError::InvalidInput {
            field,
            reason: format!("visible length must be between {min_chars} and {max_chars}"),
        })
    }
}

fn stable_hex(value: &str) -> String {
    let mut hash = FNV_OFFSET;
    for byte in value.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:08x}")
}
