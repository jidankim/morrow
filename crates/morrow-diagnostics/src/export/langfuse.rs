use std::path::Path;

use crate::export::{read_trace_input, write_trace_export_json_file, ExportReadError};
use crate::trace::{TraceOperation, TraceRecord};

mod model;
pub use model::*;

const PAYLOAD_SCHEMA: &str = "morrow.langfuse_payload.v1";
const SOURCE_NAME: &str = "morrow-local-trace-jsonl";
const OWNERSHIP_NOTE: &str = "morrow_local_traces_are_canonical";
const MASKING_POLICY: &str = "morrow_metadata_only_v1";

#[derive(Debug, thiserror::Error)]
pub enum LangfuseExportError {
    #[error("langfuse export read failed")]
    Read(#[from] ExportReadError),
    #[error("langfuse backend unavailable")]
    BackendUnavailable { receipt: LangfuseBackendReceipt },
}

pub fn export_langfuse_payload(input: &Path) -> Result<LangfusePayload, LangfuseExportError> {
    let result = read_trace_input(input)?;
    let mut traces = Vec::new();
    for record in &result.records {
        push_observation(&mut traces, record);
    }
    Ok(LangfusePayload {
        schema_version: PAYLOAD_SCHEMA,
        source: SOURCE_NAME,
        ownership: OWNERSHIP_NOTE,
        traces,
        export: LangfuseExportReceipt {
            records_read: result.records.len(),
            skipped_corrupt_lines: result.skipped_lines,
            masking_policy: MASKING_POLICY,
            dropped_categories: dropped_categories(),
            backend: LangfuseBackendReceipt::not_configured(),
        },
    })
}

pub fn write_langfuse_payload(
    input: &Path,
    out: &Path,
) -> Result<LangfusePayload, LangfuseExportError> {
    let payload = export_langfuse_payload(input)?;
    write_trace_export_json_file(input, out, &payload)?;
    Ok(payload)
}

pub fn prove_langfuse_backend_absent(
    config: &LangfuseBackendConfig,
) -> Result<LangfuseBackendReceipt, LangfuseExportError> {
    Err(LangfuseExportError::BackendUnavailable {
        receipt: LangfuseBackendReceipt::from_config(config),
    })
}

impl LangfuseBackendReceipt {
    fn not_configured() -> Self {
        Self {
            status: LangfuseBackendStatus::NotConfigured,
            network_attempted: false,
            endpoint_configured: false,
            credentials_configured: false,
            detail: "langfuse backend is not configured; capture is local only",
        }
    }

    fn from_config(config: &LangfuseBackendConfig) -> Self {
        let endpoint_configured = config.endpoint.is_some();
        let credentials_configured = config.public_key.is_some() && config.secret_key.is_some();
        let status = if !endpoint_configured {
            LangfuseBackendStatus::NotConfigured
        } else if !credentials_configured {
            LangfuseBackendStatus::MissingCredentials
        } else {
            LangfuseBackendStatus::OfflineOnly
        };
        Self {
            status,
            network_attempted: false,
            endpoint_configured,
            credentials_configured,
            detail: "network export is intentionally disabled in the dev-only adapter",
        }
    }
}

fn push_observation(traces: &mut Vec<LangfuseTrace>, record: &TraceRecord) {
    let trace_index = match traces
        .iter()
        .position(|trace| trace.id == record.trace.trace_id)
    {
        Some(index) => index,
        None => {
            traces.push(LangfuseTrace::from_record(record));
            traces.len() - 1
        }
    };
    if let Some(trace) = traces.get_mut(trace_index) {
        trace
            .observations
            .push(LangfuseObservation::from_record(record));
    }
}

impl LangfuseTrace {
    fn from_record(record: &TraceRecord) -> Self {
        Self {
            id: record.trace.trace_id.clone(),
            name: format!("morrow.{}", operation_name(record.span.operation)),
            timestamp: record.span.started_at.clone(),
            metadata: LangfuseTraceMetadata {
                source_trace_id: record.trace.trace_id.clone(),
                source: SOURCE_NAME,
            },
            observations: Vec::new(),
        }
    }
}

impl LangfuseObservation {
    fn from_record(record: &TraceRecord) -> Self {
        Self {
            id: record.trace.span_id.clone(),
            trace_id: record.trace.trace_id.clone(),
            parent_observation_id: record.trace.parent_span_id.clone(),
            name: operation_name(record.span.operation).to_owned(),
            kind: "span",
            start_time: record.span.started_at.clone(),
            end_time: record.span.ended_at.clone(),
            input: MaskedValue::masked(),
            output: MaskedValue::masked(),
            metadata: LangfuseObservationMetadata::from_record(record),
        }
    }
}

impl MaskedValue {
    fn masked() -> Self {
        Self {
            status: "masked",
            policy: MASKING_POLICY,
        }
    }
}

impl LangfuseObservationMetadata {
    fn from_record(record: &TraceRecord) -> Self {
        Self {
            component: record.span.component,
            operation: record.span.operation,
            decision: record.span.decision,
            outcome: record.span.outcome,
            reason_code: record.span.reason_code.clone(),
            provider_id: record.span.provider_id.clone(),
            model_id: record.span.model_id.clone(),
            confidence_millis: record.span.confidence_millis,
            title_status: record.span.title_status.clone(),
            privacy_tier: record.span.privacy_tier,
            classifier_stage: record.span.classifier_stage.clone(),
            router_stage: record.span.router_stage.clone(),
            ood_score_millis: record.span.ood_score_millis,
            replay_run_id: record.span.replay_run_id.clone(),
            dropped: DroppedMetadata {
                policy: MASKING_POLICY,
                categories: dropped_categories(),
            },
        }
    }
}

fn dropped_categories() -> Vec<DroppedCategory> {
    vec![
        DroppedCategory::UserContent,
        DroppedCategory::VendorPayload,
        DroppedCategory::VectorValues,
        DroppedCategory::NativeIdentifier,
        DroppedCategory::TitleContent,
    ]
}

const fn operation_name(operation: TraceOperation) -> &'static str {
    match operation {
        TraceOperation::ParserDecision => "parser_decision",
        TraceOperation::ProviderRoute => "provider_route",
        TraceOperation::ProviderResult => "provider_result",
        TraceOperation::SchemaValidation => "schema_validation",
        TraceOperation::ThresholdDecision => "threshold_decision",
        TraceOperation::OutcomeMaterialized => "outcome_materialized",
        TraceOperation::ClassifierScore => "classifier_score",
        TraceOperation::RouterDecision => "router_decision",
        TraceOperation::OodCheck => "ood_check",
        TraceOperation::PollEmpty => "poll_empty",
        TraceOperation::CursorAdvanced => "cursor_advanced",
        TraceOperation::UserCorrection => "user_correction",
        TraceOperation::CandidateSuperseded => "candidate_superseded",
        TraceOperation::CandidateRescheduled => "candidate_rescheduled",
        TraceOperation::CandidateCancelled => "candidate_cancelled",
        TraceOperation::CalendarDryRun => "calendar_dry_run",
        TraceOperation::CalendarCommitIdempotency => "calendar_commit_idempotency",
        TraceOperation::ReplayRun => "replay_run",
    }
}
