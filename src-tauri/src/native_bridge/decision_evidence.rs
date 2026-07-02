use std::path::Path;

use morrow_diagnostics::{diagnostics_trace_dir, TraceReader, TraceRecord};
use morrow_storage::{CandidateId, DecisionEvidenceSubjectType, DecisionEvidenceSummary, Store};
use serde::{Deserialize, Serialize};

#[path = "decision_evidence_trace_names.rs"]
mod decision_evidence_trace_names;

use super::scan::LatestEvalStatus;
use decision_evidence_trace_names::{
    trace_component_name, trace_decision_name, trace_operation_name, trace_outcome_name,
};

const DEFAULT_EVIDENCE_LIMIT: usize = 20;
const MAX_EVIDENCE_LIMIT: usize = 50;
const MAX_TRACE_SEQUENCE: usize = 8;
const STORE_UNAVAILABLE_ERROR: &str = "decision evidence store unavailable";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LoadDecisionEvidenceRequest {
    #[serde(default = "default_evidence_limit")]
    pub limit: usize,
    #[serde(default)]
    pub created_candidate_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DecisionEvidenceReport {
    pub items: Vec<DecisionEvidenceItem>,
    pub skipped_trace_line_count: usize,
    pub latest_eval_status: LatestEvalStatus,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DecisionEvidenceItem {
    pub subject_type: NativeDecisionEvidenceSubjectType,
    pub candidate_id: Option<String>,
    pub candidate_state: Option<String>,
    pub candidate_kind: Option<String>,
    pub route: Option<String>,
    pub reason_code: Option<String>,
    pub confidence_millis: Option<i64>,
    pub label_type: String,
    pub label_value: String,
    pub source_excerpt_policy: String,
    pub privacy_tier: String,
    pub has_diagnostics_hashes: bool,
    pub created_at: i64,
    pub trace_retention: DecisionEvidenceTraceRetention,
    pub trace_sequence: Vec<DecisionTraceStep>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NativeDecisionEvidenceSubjectType {
    Candidate,
    QuietLog,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DecisionEvidenceTraceRetention {
    Retained,
    NotRetained,
    DiagnosticsMissing,
    TraceMissing,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DecisionTraceStep {
    pub component: String,
    pub operation: String,
    pub decision: Option<String>,
    pub outcome: String,
}

pub fn load_decision_evidence_at(
    store_path: &Path,
    app_data_dir: &Path,
    request: LoadDecisionEvidenceRequest,
) -> Result<DecisionEvidenceReport, String> {
    let store = Store::open(store_path).map_err(|_| store_unavailable_error())?;
    let limit = request.limit.min(MAX_EVIDENCE_LIMIT);
    let created_candidate_ids = parse_created_candidate_ids(&request.created_candidate_ids)?;
    let summaries = match created_candidate_ids.is_empty() {
        true => store.recent_decision_evidence(limit),
        false => store.decision_evidence_for_candidate_ids(&created_candidate_ids, limit),
    }
    .map_err(|_| store_unavailable_error())?;
    let counts = store
        .feedback_eval_counts()
        .map_err(|_| store_unavailable_error())?;
    let traces = read_traces(app_data_dir);
    let items = summaries
        .iter()
        .map(|summary| item_from_summary(summary, &traces))
        .collect();
    Ok(DecisionEvidenceReport {
        items,
        skipped_trace_line_count: traces.skipped_lines,
        latest_eval_status: LatestEvalStatus::from(counts.latest_eval_status),
    })
}

fn parse_created_candidate_ids(candidate_ids: &[String]) -> Result<Vec<CandidateId>, String> {
    candidate_ids
        .iter()
        .map(|candidate_id| {
            CandidateId::from_storage(candidate_id)
                .map_err(|_| "invalid decision evidence request".to_owned())
        })
        .collect()
}

fn item_from_summary(
    summary: &DecisionEvidenceSummary,
    traces: &TraceReadSummary,
) -> DecisionEvidenceItem {
    let (trace_retention, trace_sequence) = trace_attachment(summary, traces);
    DecisionEvidenceItem {
        subject_type: subject_type(summary.subject_type),
        candidate_id: summary
            .candidate_id
            .as_ref()
            .map(|candidate_id| candidate_id.as_str().to_owned()),
        candidate_state: summary
            .candidate_state
            .map(|state| state.as_str().to_owned()),
        candidate_kind: summary.candidate_kind.map(|kind| kind.as_str().to_owned()),
        route: summary.route.clone(),
        reason_code: summary.reason_code.clone(),
        confidence_millis: summary.confidence_millis,
        label_type: summary.label_type.as_str().to_owned(),
        label_value: summary.label_value.as_str().to_owned(),
        source_excerpt_policy: summary.source_excerpt_policy.as_str().to_owned(),
        privacy_tier: summary.privacy_tier.as_str().to_owned(),
        has_diagnostics_hashes: summary.diagnostics_chat_hash_present
            || summary.diagnostics_message_hash_present,
        created_at: summary.created_at,
        trace_retention,
        trace_sequence,
    }
}

fn trace_attachment(
    summary: &DecisionEvidenceSummary,
    traces: &TraceReadSummary,
) -> (DecisionEvidenceTraceRetention, Vec<DecisionTraceStep>) {
    let (Some(trace_id), Some(span_id)) = (
        summary.diagnostics_trace_id.as_deref(),
        summary.diagnostics_span_id.as_deref(),
    ) else {
        return (DecisionEvidenceTraceRetention::NotRetained, Vec::new());
    };

    match traces.availability {
        TraceAvailability::Disabled => (DecisionEvidenceTraceRetention::NotRetained, Vec::new()),
        TraceAvailability::Missing => (
            DecisionEvidenceTraceRetention::DiagnosticsMissing,
            Vec::new(),
        ),
        TraceAvailability::Readable => retained_trace_sequence(trace_id, span_id, &traces.records),
    }
}

fn retained_trace_sequence(
    trace_id: &str,
    span_id: &str,
    records: &[TraceRecord],
) -> (DecisionEvidenceTraceRetention, Vec<DecisionTraceStep>) {
    let matched_span = records
        .iter()
        .any(|record| record.trace.trace_id == trace_id && record.trace.span_id == span_id);
    if !matched_span {
        return (DecisionEvidenceTraceRetention::TraceMissing, Vec::new());
    }
    let sequence = records
        .iter()
        .filter(|record| record.trace.trace_id == trace_id)
        .take(MAX_TRACE_SEQUENCE)
        .map(trace_step)
        .collect();
    (DecisionEvidenceTraceRetention::Retained, sequence)
}

fn trace_step(record: &TraceRecord) -> DecisionTraceStep {
    DecisionTraceStep {
        component: trace_component_name(record.span.component).to_owned(),
        operation: trace_operation_name(record.span.operation).to_owned(),
        decision: record
            .span
            .decision
            .map(|decision| trace_decision_name(decision).to_owned()),
        outcome: trace_outcome_name(record.span.outcome).to_owned(),
    }
}

#[derive(Debug)]
struct TraceReadSummary {
    availability: TraceAvailability,
    records: Vec<TraceRecord>,
    skipped_lines: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TraceAvailability {
    Readable,
    Disabled,
    Missing,
}

fn read_traces(app_data_dir: &Path) -> TraceReadSummary {
    let traces_dir = diagnostics_trace_dir(app_data_dir);
    if !traces_dir.exists() {
        let availability = match app_data_dir.exists() {
            true => TraceAvailability::Missing,
            false => TraceAvailability::Disabled,
        };
        return TraceReadSummary {
            availability,
            records: Vec::new(),
            skipped_lines: 0,
        };
    }
    match TraceReader::new(&traces_dir).read_all() {
        Ok(result) => TraceReadSummary {
            availability: TraceAvailability::Readable,
            records: result.records,
            skipped_lines: result.skipped_lines,
        },
        Err(_) => TraceReadSummary {
            availability: TraceAvailability::Missing,
            records: Vec::new(),
            skipped_lines: 0,
        },
    }
}

fn subject_type(subject_type: DecisionEvidenceSubjectType) -> NativeDecisionEvidenceSubjectType {
    match subject_type {
        DecisionEvidenceSubjectType::Candidate => NativeDecisionEvidenceSubjectType::Candidate,
        DecisionEvidenceSubjectType::QuietLog => NativeDecisionEvidenceSubjectType::QuietLog,
    }
}

const fn default_evidence_limit() -> usize {
    DEFAULT_EVIDENCE_LIMIT
}

fn store_unavailable_error() -> String {
    STORE_UNAVAILABLE_ERROR.to_owned()
}
