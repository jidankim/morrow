use morrow_messages::MessageEvidence;
use morrow_storage::CandidateKind;
use serde::Deserialize;

use crate::schema::{evidence_guid_for_id, ProviderCandidatePayload, SchemaRejection};
use crate::types::{
    CivilDateTime, DetectionConfig, ListReminderDefaultDueMode, ListReminderDefaultDueTime,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ListReminderItem {
    pub(super) name: String,
    pub(super) quantity: f64,
    #[serde(default)]
    pub(super) unit: Option<String>,
    pub(super) evidence_ids: Vec<String>,
}

pub(super) fn default_list_reminder_due_time(
    payload: &ProviderCandidatePayload,
    config: &DetectionConfig,
) -> Option<CivilDateTime> {
    if payload.items.is_none()
        || !config.profile.enabled
        || config.profile.default_due_mode != ListReminderDefaultDueMode::NextLocalDayAtDefaultTime
    {
        return None;
    }
    match config.profile.default_due_time {
        ListReminderDefaultDueTime::TwentyThreeFiftyNine => {
            Some(next_day_at(config.reference.observed, 23, 59))
        }
    }
}

pub(super) fn validate_list_item_evidence(
    items: Option<&[ListReminderItem]>,
    evidence: &[MessageEvidence],
) -> Result<(), SchemaRejection> {
    let Some(items) = items else {
        return Ok(());
    };
    if items.is_empty() || items.len() > 20 {
        return Err(SchemaRejection::InvalidSchema);
    }
    for item in items {
        if item.evidence_ids.is_empty() {
            return Err(SchemaRejection::InvalidSchema);
        }
        for evidence_id in &item.evidence_ids {
            evidence_guid_for_id(evidence, evidence_id)?;
        }
    }
    Ok(())
}

pub(super) fn rendered_list_title(
    kind: CandidateKind,
    items: &[ListReminderItem],
) -> Result<String, SchemaRejection> {
    if kind != CandidateKind::TaskReminder {
        return Err(SchemaRejection::InvalidSchema);
    }
    let mut rendered_items = Vec::with_capacity(items.len());
    for item in items {
        rendered_items.push(rendered_list_item(item)?);
    }
    let title = format!("Daily list: {}", rendered_items.join("; "));
    if title.len() > 160 {
        return Err(SchemaRejection::InvalidSchema);
    }
    Ok(title)
}

fn rendered_list_item(item: &ListReminderItem) -> Result<String, SchemaRejection> {
    let quantity = normalized_quantity(item.quantity)?;
    let name = normalized_visible_text(&item.name, 5, 80)?;
    match &item.unit {
        Some(unit) => {
            let normalized_unit = normalized_visible_text(unit, 3, 24)?;
            Ok(format!("{quantity} {normalized_unit} {name}"))
        }
        None => Ok(format!("{quantity} {name}")),
    }
}

fn normalized_quantity(quantity: f64) -> Result<String, SchemaRejection> {
    if !quantity.is_finite() || quantity <= 0.0 || quantity > 999.0 || quantity.fract() != 0.0 {
        return Err(SchemaRejection::InvalidSchema);
    }
    Ok(format!("{quantity:.0}"))
}

fn normalized_visible_text(
    raw: &str,
    max_words: usize,
    max_bytes: usize,
) -> Result<String, SchemaRejection> {
    let trimmed = raw.trim();
    let word_count = trimmed.split_whitespace().count();
    if trimmed.is_empty()
        || trimmed.len() > max_bytes
        || word_count == 0
        || word_count > max_words
        || trimmed
            .chars()
            .any(|ch| ch.is_control() || matches!(ch, ';' | '[' | ']'))
    {
        return Err(SchemaRejection::InvalidSchema);
    }
    Ok(trimmed.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn next_day_at(value: CivilDateTime, hour: u8, minute: u8) -> CivilDateTime {
    let max_day = days_in_month(value.year, value.month).unwrap_or(31);
    if value.day < max_day {
        return CivilDateTime {
            year: value.year,
            month: value.month,
            day: value.day + 1,
            hour,
            minute,
        };
    }
    if value.month < 12 {
        return CivilDateTime {
            year: value.year,
            month: value.month + 1,
            day: 1,
            hour,
            minute,
        };
    }
    CivilDateTime {
        year: value.year + 1,
        month: 1,
        day: 1,
        hour,
        minute,
    }
}

const fn days_in_month(year: u16, month: u8) -> Option<u8> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 if is_leap_year(year) => Some(29),
        2 => Some(28),
        _ => None,
    }
}

const fn is_leap_year(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}
