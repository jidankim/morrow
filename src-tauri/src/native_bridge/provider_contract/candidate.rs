use morrow_messages::MessageEvidence;
use morrow_storage::{validate_normalized_time as storage_validate_normalized_time, CandidateKind};
use serde::Deserialize;
use serde_json::{json, Value};

use super::{normalized_time_schema, ProviderContractError};

pub(crate) const PROVIDER_CANDIDATE_SCHEMA_VERSION: &str = "provider-candidate-schema-v3";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateObject {
    kind: String,
    title: String,
    confidence_millis: i64,
    normalized_time: String,
    anchor_evidence_id: String,
    evidence_ids: Vec<String>,
    #[serde(default)]
    items: Option<Vec<ListReminderItem>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListReminderItem {
    name: String,
    quantity: f64,
    #[serde(default)]
    unit: Option<String>,
    evidence_ids: Vec<String>,
}

pub(crate) fn candidate_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "kind",
            "title",
            "confidence_millis",
            "normalized_time",
            "anchor_evidence_id",
            "evidence_ids",
            "items"
        ],
        "properties": {
            "kind": {
                "type": "string",
                "enum": [
                    "calendar_event",
                    "task_reminder",
                    "event_update",
                    "event_reschedule",
                    "event_cancellation",
                    "reminder_update",
                    "reminder_reschedule",
                    "reminder_cancellation"
                ]
            },
            "title": { "type": "string" },
            "confidence_millis": { "type": "integer", "minimum": 0, "maximum": 1000 },
            "normalized_time": normalized_time_schema(),
            "anchor_evidence_id": { "type": "string" },
            "evidence_ids": {
                "type": "array",
                "minItems": 1,
                "items": { "type": "string" }
            },
            "items": {
                "type": ["array", "null"],
                "description": "Optional list-reminders-v1 structured item data. Use only for one task_reminder created from a quantity list; this data is validation-only and is not persisted as separate rows.",
                "minItems": 1,
                "maxItems": 20,
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["name", "quantity", "unit", "evidence_ids"],
                    "properties": {
                        "name": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": 80,
                            "description": "Normalized item name, 1 to 5 visible words."
                        },
                        "quantity": {
                            "type": "number",
                            "exclusiveMinimum": 0,
                            "maximum": 999
                        },
                        "unit": {
                            "type": ["string", "null"],
                            "minLength": 1,
                            "maxLength": 24,
                            "description": "Optional normalized unit, at most 3 visible words."
                        },
                        "evidence_ids": {
                            "type": "array",
                            "minItems": 1,
                            "items": { "type": "string" }
                        }
                    }
                }
            }
        }
    })
}

pub(crate) fn localize_candidate_json(
    text: &str,
    evidence: &[MessageEvidence],
) -> Result<String, ProviderContractError> {
    let value: Value = serde_json::from_str(text)
        .map_err(|_| invalid_candidate("candidate text was not valid json"))?;
    if !value.is_object() {
        return Err(invalid_candidate("candidate text was not an object"));
    }
    let candidate: CandidateObject = serde_json::from_value(value)
        .map_err(|_| invalid_candidate("candidate object did not match schema"))?;
    let kind = CandidateKind::parse(&candidate.kind)
        .map_err(|_| invalid_candidate("candidate kind was invalid"))?;
    if candidate.title.is_empty() || candidate.title.len() > 160 {
        return Err(invalid_candidate("candidate title was invalid"));
    }
    if candidate.confidence_millis < 0 || candidate.confidence_millis > 1000 {
        return Err(invalid_candidate("candidate confidence was invalid"));
    }
    validate_normalized_time(&candidate.normalized_time)?;
    if candidate.anchor_evidence_id.is_empty() || candidate.evidence_ids.is_empty() {
        return Err(invalid_candidate("candidate evidence was invalid"));
    }
    let anchor_message_guid = evidence_guid_for_id(evidence, &candidate.anchor_evidence_id)?;
    let evidence_message_guids = candidate
        .evidence_ids
        .iter()
        .map(|id| evidence_guid_for_id(evidence, id))
        .collect::<Result<Vec<_>, _>>()?;
    validate_list_item_evidence(candidate.items.as_deref(), evidence)?;
    let title = match candidate.items.as_deref() {
        Some(items) => rendered_list_title(kind, items)?,
        None => candidate.title,
    };
    Ok(json!({
        "kind": candidate.kind,
        "title": title,
        "confidence_millis": candidate.confidence_millis,
        "normalized_time": candidate.normalized_time,
        "anchor_message_guid": anchor_message_guid,
        "evidence_message_guids": evidence_message_guids
    })
    .to_string())
}

fn validate_list_item_evidence(
    items: Option<&[ListReminderItem]>,
    evidence: &[MessageEvidence],
) -> Result<(), ProviderContractError> {
    let Some(items) = items else {
        return Ok(());
    };
    if items.is_empty() || items.len() > 20 {
        return Err(invalid_candidate("candidate list items were invalid"));
    }
    for item in items {
        if item.evidence_ids.is_empty() {
            return Err(invalid_candidate(
                "candidate list item evidence was invalid",
            ));
        }
        for evidence_id in &item.evidence_ids {
            evidence_guid_for_id(evidence, evidence_id)?;
        }
    }
    Ok(())
}

fn rendered_list_title(
    kind: CandidateKind,
    items: &[ListReminderItem],
) -> Result<String, ProviderContractError> {
    if kind != CandidateKind::TaskReminder {
        return Err(invalid_candidate("candidate list kind was invalid"));
    }
    let mut rendered_items = Vec::with_capacity(items.len());
    for item in items {
        rendered_items.push(rendered_list_item(item)?);
    }
    let title = format!("Daily list: {}", rendered_items.join("; "));
    if title.len() > 160 {
        return Err(invalid_candidate("candidate list title was invalid"));
    }
    Ok(title)
}

fn rendered_list_item(item: &ListReminderItem) -> Result<String, ProviderContractError> {
    let quantity = normalized_quantity(item.quantity)?;
    let name = normalized_visible_text(&item.name, 5, 80, "candidate list item name was invalid")?;
    match &item.unit {
        Some(unit) => {
            let normalized_unit =
                normalized_visible_text(unit, 3, 24, "candidate list item unit was invalid")?;
            Ok(format!("{quantity} {normalized_unit} {name}"))
        }
        None => Ok(format!("{quantity} {name}")),
    }
}

fn normalized_quantity(quantity: f64) -> Result<String, ProviderContractError> {
    if !quantity.is_finite() || quantity <= 0.0 || quantity > 999.0 || quantity.fract() != 0.0 {
        return Err(invalid_candidate(
            "candidate list item quantity was invalid",
        ));
    }
    Ok(format!("{quantity:.0}"))
}

fn normalized_visible_text(
    raw: &str,
    max_words: usize,
    max_bytes: usize,
    reason: &'static str,
) -> Result<String, ProviderContractError> {
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
        return Err(invalid_candidate(reason));
    }
    Ok(trimmed.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn validate_normalized_time(raw: &str) -> Result<(), ProviderContractError> {
    storage_validate_normalized_time(raw)
        .map_err(|_| invalid_candidate("candidate normalized_time was invalid"))
}

fn evidence_guid_for_id(
    evidence: &[MessageEvidence],
    evidence_id: &str,
) -> Result<String, ProviderContractError> {
    let index_text = evidence_id
        .strip_prefix("evidence://selected/")
        .ok_or_else(|| invalid_candidate("candidate evidence was hallucinated"))?;
    let index = index_text
        .parse::<usize>()
        .map_err(|_| invalid_candidate("candidate evidence was hallucinated"))?;
    evidence
        .get(index)
        .map(|message| message.message_guid.as_str().to_owned())
        .ok_or_else(|| invalid_candidate("candidate evidence was hallucinated"))
}

const fn invalid_candidate(reason: &'static str) -> ProviderContractError {
    ProviderContractError::InvalidCandidate { reason }
}
