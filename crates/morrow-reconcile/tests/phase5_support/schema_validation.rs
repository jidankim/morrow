use std::collections::BTreeSet;

use morrow_storage::{FeedbackLabelType, FeedbackLabelValue, FeedbackPrivacyTier};

use super::schema::{
    SchemaValidationError, TrajectoryCase, FORBIDDEN_FIELD_NAMES, FORBIDDEN_TEXT_MARKERS,
    MAX_PLACEHOLDER_LEN, SCORE_TOTAL,
};

pub(crate) fn validate_trajectory_cases(
    cases: &[TrajectoryCase],
) -> Result<(), SchemaValidationError> {
    let mut ids = BTreeSet::new();
    for case in cases {
        validate_case(case)?;
        if !ids.insert(case.trajectory_case_id) {
            return Err(SchemaValidationError::new(format!(
                "duplicate trajectory_case_id {}",
                case.trajectory_case_id
            )));
        }
    }
    Ok(())
}

fn validate_case(case: &TrajectoryCase) -> Result<(), SchemaValidationError> {
    require_stable_id("trajectory_case_id", case.trajectory_case_id)?;
    require_stable_id("source_fixture_id", case.source_fixture_id)?;
    require_stable_id("candidate_fixture_id", case.candidate_fixture_id)?;
    require_stable_id("proposal_fixture_id", case.proposal_fixture_id)?;
    require_placeholder(case)?;
    require_non_empty("expected_outcomes", case.expected_outcomes)?;
    require_non_empty("expected_labels", case.expected_labels)?;
    require_labels(case)?;
    require_non_empty("expected_trace_operations", case.expected_trace_operations)?;
    require_trace_operations(case)?;
    require_non_empty("allowed_side_effects", case.allowed_side_effects)?;
    require_cleanup(case)?;
    require_score_total(case)?;
    reject_forbidden_serialized_shape(case)
}

fn require_stable_id(field: &str, value: &str) -> Result<(), SchemaValidationError> {
    if value.starts_with("phase5:")
        && value.ends_with(":v1")
        && !value.contains(char::is_whitespace)
    {
        return Ok(());
    }
    Err(SchemaValidationError::new(format!(
        "{field} must be a stable opaque phase5 id"
    )))
}

fn require_placeholder(case: &TrajectoryCase) -> Result<(), SchemaValidationError> {
    let value = case.bounded_excerpt_placeholder;
    if value.len() <= MAX_PLACEHOLDER_LEN && value.starts_with('[') && value.ends_with(']') {
        return Ok(());
    }
    Err(SchemaValidationError::new(format!(
        "case {} excerpt must be a bounded placeholder",
        case.trajectory_case_id
    )))
}

fn require_non_empty<T>(field: &str, values: &[T]) -> Result<(), SchemaValidationError> {
    if values.is_empty() {
        return Err(SchemaValidationError::new(format!(
            "{field} must not be empty"
        )));
    }
    Ok(())
}

fn require_labels(case: &TrajectoryCase) -> Result<(), SchemaValidationError> {
    for label in case.expected_labels {
        let label_type = FeedbackLabelType::parse(label.label_type).map_err(|err| {
            SchemaValidationError::new(format!(
                "case {} invalid label type: {err}",
                case.trajectory_case_id
            ))
        })?;
        FeedbackLabelValue::parse(label_type, label.label_value, "label_value").map_err(|err| {
            SchemaValidationError::new(format!(
                "case {} invalid label value: {err}",
                case.trajectory_case_id
            ))
        })?;
    }
    Ok(())
}

fn require_trace_operations(case: &TrajectoryCase) -> Result<(), SchemaValidationError> {
    for operation in case.expected_trace_operations {
        if operation.component.is_empty()
            || operation.operation.is_empty()
            || operation.outcome.is_empty()
        {
            return Err(SchemaValidationError::new(format!(
                "case {} trace operation fields must not be empty",
                case.trajectory_case_id
            )));
        }
        if operation.privacy_tier != FeedbackPrivacyTier::InternalMetadata {
            return Err(SchemaValidationError::new(format!(
                "case {} trace operation {} must use sanitized internal metadata",
                case.trajectory_case_id, operation.operation
            )));
        }
    }
    Ok(())
}

fn require_cleanup(case: &TrajectoryCase) -> Result<(), SchemaValidationError> {
    match case.cleanup {
        Some(cleanup) if !cleanup.live_surface_used => Ok(()),
        Some(_) => Err(SchemaValidationError::new(format!(
            "case {} must not use live surfaces",
            case.trajectory_case_id
        ))),
        None => Err(SchemaValidationError::new(format!(
            "case {} missing cleanup expectation",
            case.trajectory_case_id
        ))),
    }
}

fn require_score_total(case: &TrajectoryCase) -> Result<(), SchemaValidationError> {
    let total = match case.explicit_score_total {
        Some(total) => total,
        None => case
            .score_weights
            .iter()
            .fold(0_u16, |sum, weight| sum.saturating_add(weight.weight)),
    };
    if total == SCORE_TOTAL {
        return Ok(());
    }
    Err(SchemaValidationError::new(format!(
        "case {} score total must be {SCORE_TOTAL}, got {total}",
        case.trajectory_case_id
    )))
}

fn reject_forbidden_serialized_shape(case: &TrajectoryCase) -> Result<(), SchemaValidationError> {
    let cleanup = case.cleanup.ok_or_else(|| {
        SchemaValidationError::new(format!(
            "case {} missing cleanup expectation",
            case.trajectory_case_id
        ))
    })?;
    let value = serde_json::json!({
        "trajectory_case_id": case.trajectory_case_id,
        "family": case.family,
        "source_fixture_id": case.source_fixture_id,
        "candidate_fixture_id": case.candidate_fixture_id,
        "proposal_fixture_id": case.proposal_fixture_id,
        "bounded_excerpt_placeholder": case.bounded_excerpt_placeholder,
        "expected_candidate_kind": case.expected_candidate_kind.as_str(),
        "expected_proposal_kind": case.expected_proposal_kind.as_str(),
        "expected_outcomes": case.expected_outcomes.iter().map(|outcome| outcome.as_str()).collect::<Vec<_>>(),
        "expected_labels": case.expected_labels.iter().map(|label| serde_json::json!({
            "label_type": label.label_type,
            "label_value": label.label_value,
        })).collect::<Vec<_>>(),
        "expected_trace_operations": case.expected_trace_operations.iter().map(|operation| serde_json::json!({
            "component": operation.component,
            "operation": operation.operation,
            "outcome": operation.outcome,
            "privacy_tier": operation.privacy_tier.as_str(),
        })).collect::<Vec<_>>(),
        "allowed_side_effects": case.allowed_side_effects.iter().map(|effect| effect.as_str()).collect::<Vec<_>>(),
        "cleanup": {
            "action": cleanup.action.as_str(),
            "non_target_fixture_id": cleanup.non_target_fixture_id,
            "live_surface_used": cleanup.live_surface_used,
        },
        "score_weights": case.score_weights.iter().map(|weight| serde_json::json!({
            "category": weight.category,
            "weight": weight.weight,
        })).collect::<Vec<_>>(),
        "explicit_score_total": case.explicit_score_total,
    });
    reject_forbidden_value(&value, "")
}

fn reject_forbidden_value(
    value: &serde_json::Value,
    path: &str,
) -> Result<(), SchemaValidationError> {
    match value {
        serde_json::Value::Object(fields) => {
            for (key, child) in fields {
                if FORBIDDEN_FIELD_NAMES.contains(&key.as_str()) {
                    return Err(SchemaValidationError::new(format!(
                        "forbidden raw field {key} at {path}"
                    )));
                }
                reject_forbidden_value(child, key)?;
            }
        }
        serde_json::Value::Array(items) => {
            for child in items {
                reject_forbidden_value(child, path)?;
            }
        }
        serde_json::Value::String(text) => reject_forbidden_text(text)?,
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {}
    }
    Ok(())
}

fn reject_forbidden_text(text: &str) -> Result<(), SchemaValidationError> {
    for marker in FORBIDDEN_TEXT_MARKERS {
        if text.contains(marker) {
            return Err(SchemaValidationError::new(format!(
                "forbidden raw marker {marker}"
            )));
        }
    }
    Ok(())
}
