use crate::{TraceComponent, TraceDecision, TraceOperation, TraceOutcome, TraceRecord};

const PRIVATE_MARKERS: &str = "raw_title|full_message|provider_json|native_identifier|unredacted_title|provider meeting|private title|private message|morrow_privacy_canary|/users/|\\users\\|application support|/private/var/|/var/folders/|messages/chat.db|eventkit|chat-guid|message-guid|native-id|native_id|ekevent|x-apple|com.apple";
const SANITIZED_MARKERS: &str = "redacted|hidden|hashed|sanitized|placeholder";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TrajectoryAnchorReport {
    pub trajectory_case_id: String,
    pub steps: Vec<TrajectoryAnchorStep>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TrajectoryAnchorStep {
    pub trajectory_case_id: String,
    pub feature_snapshot_id: String,
    pub trace_id: String,
    pub span_id: String,
    pub component: TraceComponent,
    pub operation: TraceOperation,
    pub decision: Option<TraceDecision>,
    pub outcome: TraceOutcome,
    pub label_type: String,
    pub label_value: String,
    pub decision_evidence_subject_id: String,
    pub decision_evidence_trace_id: String,
    pub decision_evidence_span_id: String,
}

#[derive(Debug, Clone, Copy)]
pub struct TrajectoryAnchorStepInput<'a> {
    pub trajectory_case_id: &'a str,
    pub feature_snapshot_id: &'a str,
    pub label_type: &'a str,
    pub label_value: &'a str,
    pub decision_evidence_subject_id: &'a str,
    pub record: &'a TraceRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TrajectoryAnchorError {
    #[error("trajectory anchor report has no steps")]
    MissingStep,
    #[error("trajectory anchor step {index} has mismatched case id")]
    MismatchedCaseId { index: usize },
    #[error("trajectory anchor step {index} has mismatched decision evidence trace")]
    MismatchedDecisionEvidenceTrace { index: usize },
    #[error("trajectory anchor step {index} has mismatched decision evidence span")]
    MismatchedDecisionEvidenceSpan { index: usize },
    #[error("trajectory anchor step {index} has malformed {field}")]
    MalformedField { index: usize, field: &'static str },
    #[error("trajectory anchor step {index} violates private boundary in {field}")]
    ForbiddenPrivateToken { index: usize, field: &'static str },
}

impl TrajectoryAnchorReport {
    pub fn validate(&self) -> Result<(), TrajectoryAnchorError> {
        if self.steps.is_empty() {
            return Err(TrajectoryAnchorError::MissingStep);
        }
        for (index, step) in self.steps.iter().enumerate() {
            step.validate(index, &self.trajectory_case_id)?;
        }
        Ok(())
    }
}

impl TrajectoryAnchorStep {
    pub fn from_trace_record(input: TrajectoryAnchorStepInput<'_>) -> Self {
        Self {
            trajectory_case_id: input.trajectory_case_id.to_owned(),
            feature_snapshot_id: input.feature_snapshot_id.to_owned(),
            trace_id: input.record.trace.trace_id.clone(),
            span_id: input.record.trace.span_id.clone(),
            component: input.record.span.component,
            operation: input.record.span.operation,
            decision: input.record.span.decision,
            outcome: input.record.span.outcome,
            label_type: input.label_type.to_owned(),
            label_value: input.label_value.to_owned(),
            decision_evidence_subject_id: input.decision_evidence_subject_id.to_owned(),
            decision_evidence_trace_id: input.record.trace.trace_id.clone(),
            decision_evidence_span_id: input.record.trace.span_id.clone(),
        }
    }

    fn validate(&self, index: usize, report_case_id: &str) -> Result<(), TrajectoryAnchorError> {
        if self.trajectory_case_id != report_case_id {
            return Err(TrajectoryAnchorError::MismatchedCaseId { index });
        }
        if self.trace_id != self.decision_evidence_trace_id {
            return Err(TrajectoryAnchorError::MismatchedDecisionEvidenceTrace { index });
        }
        if self.span_id != self.decision_evidence_span_id {
            return Err(TrajectoryAnchorError::MismatchedDecisionEvidenceSpan { index });
        }
        for (field, prefix, value) in [
            ("trace_id", "trace", self.trace_id.as_str()),
            ("span_id", "span", self.span_id.as_str()),
            (
                "decision_evidence_trace_id",
                "trace",
                self.decision_evidence_trace_id.as_str(),
            ),
            (
                "decision_evidence_span_id",
                "span",
                self.decision_evidence_span_id.as_str(),
            ),
        ] {
            validate_opaque_id(index, field, prefix, value)?;
        }
        for (field, value) in [
            ("trajectory_case_id", self.trajectory_case_id.as_str()),
            ("feature_snapshot_id", self.feature_snapshot_id.as_str()),
            ("trace_id", self.trace_id.as_str()),
            ("span_id", self.span_id.as_str()),
            ("label_type", self.label_type.as_str()),
            ("label_value", self.label_value.as_str()),
            (
                "decision_evidence_subject_id",
                self.decision_evidence_subject_id.as_str(),
            ),
            (
                "decision_evidence_trace_id",
                self.decision_evidence_trace_id.as_str(),
            ),
            (
                "decision_evidence_span_id",
                self.decision_evidence_span_id.as_str(),
            ),
        ] {
            self.validate_field(index, field, value)?;
        }
        validate_field_shape(
            index,
            "trajectory_case_id",
            self.trajectory_case_id
                .strip_prefix("phase5-case-")
                .is_some_and(|body| is_local_body(body, '-')),
        )?;
        validate_field_shape(
            index,
            "feature_snapshot_id",
            self.feature_snapshot_id == format!("{}-snapshot", self.trajectory_case_id),
        )?;
        validate_field_shape(index, "label_type", is_token_body(&self.label_type))?;
        validate_field_shape(index, "label_value", is_token_body(&self.label_value))?;
        validate_field_shape(
            index,
            "decision_evidence_subject_id",
            self.decision_evidence_subject_id
                .strip_prefix("candidate_")
                .or_else(|| self.decision_evidence_subject_id.strip_prefix("quiet_"))
                .is_some_and(|body| is_local_body(body, '_')),
        )?;
        Ok(())
    }

    fn validate_field(
        &self,
        index: usize,
        field: &'static str,
        value: &str,
    ) -> Result<(), TrajectoryAnchorError> {
        if value.is_empty() {
            return Err(TrajectoryAnchorError::MalformedField { index, field });
        }
        if contains_forbidden_private_token(value) {
            return Err(TrajectoryAnchorError::ForbiddenPrivateToken { index, field });
        }
        Ok(())
    }
}

fn validate_field_shape(
    index: usize,
    field: &'static str,
    valid: bool,
) -> Result<(), TrajectoryAnchorError> {
    valid
        .then_some(())
        .ok_or(TrajectoryAnchorError::MalformedField { index, field })
}

fn validate_opaque_id(
    index: usize,
    field: &'static str,
    prefix: &str,
    value: &str,
) -> Result<(), TrajectoryAnchorError> {
    validate_field_shape(
        index,
        field,
        value
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_prefix('_'))
            .is_some_and(|suffix| {
                suffix.len() == 32
                    && suffix
                        .chars()
                        .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
            }),
    )
}

fn is_local_body(value: &str, separator: char) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == separator)
        && !value.starts_with(separator)
        && !value.ends_with(separator)
}

fn is_token_body(value: &str) -> bool {
    is_local_body(value, '_')
}

fn contains_forbidden_private_token(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    contains_marker(&lower, PRIVATE_MARKERS)
        || looks_like_email(value)
        || looks_like_phone(value)
        || value.trim_start().starts_with(['{', '['])
        || lower.contains("provider_payload")
        || lower.contains("raw_payload")
        || lower.contains("provider json")
        || lower.starts_with("title=")
        || lower.starts_with("title:")
        || lower.contains(" title=")
        || looks_like_raw_message(value, &lower)
}

fn contains_marker(value: &str, markers: &str) -> bool {
    markers.split('|').any(|token| value.contains(token))
}

fn looks_like_email(value: &str) -> bool {
    value.split_once('@').is_some_and(|(local, domain)| {
        !local.is_empty()
            && domain.contains('.')
            && !domain.starts_with('.')
            && !domain.ends_with('.')
            && !value.chars().any(char::is_whitespace)
    })
}

fn looks_like_phone(value: &str) -> bool {
    let digit_count = value.chars().filter(char::is_ascii_digit).count();
    digit_count >= 10
        && value
            .chars()
            .any(|ch| matches!(ch, '+' | '(' | ')' | '-' | ' ' | '.'))
        && value
            .chars()
            .all(|ch| ch.is_ascii_digit() || matches!(ch, '+' | '(' | ')' | '-' | ' ' | '.'))
}

fn looks_like_raw_message(value: &str, lower: &str) -> bool {
    !contains_marker(lower, SANITIZED_MARKERS)
        && (value.contains('\n') || lower.split_whitespace().count() >= 5)
}
