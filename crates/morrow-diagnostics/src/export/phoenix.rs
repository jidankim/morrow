use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::json;

use crate::export::{read_trace_input, write_trace_export_json_file, ExportReadError};
use crate::sink::TraceReadResult;
use crate::trace::TraceRecord;

const ADAPTER_NAME: &str = "phoenix_openinference_json";
const SOURCE_OF_TRUTH: &str = "morrow_local_trace_jsonl";
const EXPORT_FILE_NAME: &str = "phoenix-payload.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhoenixExportConfig<'a> {
    pub input_path: &'a Path,
    pub output_path: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhoenixExportReceipt {
    pub output_path: PathBuf,
    pub span_count: usize,
    pub skipped_corrupt_lines: usize,
    pub payload_bytes: u64,
}

pub fn default_phoenix_export_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir
        .join("diagnostics")
        .join("exports")
        .join("phoenix")
        .join(EXPORT_FILE_NAME)
}

pub fn export_phoenix_payload(
    config: &PhoenixExportConfig<'_>,
) -> Result<PhoenixExportReceipt, PhoenixExportError> {
    let read_result = read_trace_input(config.input_path)?;
    let payload = PhoenixPayload::from_read_result(&read_result)?;
    write_trace_export_json_file(config.input_path, config.output_path, &payload)?;
    let payload_bytes = fs::metadata(config.output_path)?.len();
    Ok(PhoenixExportReceipt {
        output_path: config.output_path.to_path_buf(),
        span_count: payload.spans.len(),
        skipped_corrupt_lines: read_result.skipped_lines,
        payload_bytes,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum PhoenixExportError {
    #[error("phoenix export io failed")]
    Io(#[from] std::io::Error),
    #[error("phoenix export read/write failed")]
    ExportIo(#[from] ExportReadError),
    #[error("phoenix export serialization failed")]
    Serialization(#[from] serde_json::Error),
    #[error("phoenix export enum value was not serialized as a string")]
    EnumString,
}

#[derive(Debug, Serialize)]
struct PhoenixPayload {
    adapter: &'static str,
    source: PhoenixSource,
    privacy: PhoenixPrivacy,
    spans: Vec<PhoenixSpan>,
}

impl PhoenixPayload {
    fn from_read_result(read_result: &TraceReadResult) -> Result<Self, PhoenixExportError> {
        let mut spans = Vec::with_capacity(read_result.records.len());
        for record in &read_result.records {
            spans.push(PhoenixSpan::from_record(record)?);
        }
        Ok(Self {
            adapter: ADAPTER_NAME,
            source: PhoenixSource::new(read_result.skipped_lines)?,
            privacy: PhoenixPrivacy::default(),
            spans,
        })
    }
}

#[derive(Debug, Serialize)]
struct PhoenixSource {
    source_of_truth: &'static str,
    trace_schema_version: String,
    skipped_corrupt_lines: usize,
    source_role: &'static str,
}

impl PhoenixSource {
    fn new(skipped_corrupt_lines: usize) -> Result<Self, PhoenixExportError> {
        Ok(Self {
            source_of_truth: SOURCE_OF_TRUTH,
            trace_schema_version: enum_string(&crate::trace::TraceSchemaVersion::V1)?,
            skipped_corrupt_lines,
            source_role: "viewer_import_capture",
        })
    }
}

#[derive(Debug, Serialize)]
struct PhoenixPrivacy {
    local_only: bool,
    network_calls: &'static str,
    identifier_policy: &'static str,
    content_policy: &'static str,
    dropped_categories: [&'static str; 6],
}

impl Default for PhoenixPrivacy {
    fn default() -> Self {
        Self {
            local_only: true,
            network_calls: "none",
            identifier_policy: "opaque_trace_and_span_ids_only",
            content_policy: "no_user_content_or_model_io",
            dropped_categories: [
                "native_identifiers",
                "user_content",
                "provider_payloads",
                "model_inputs_outputs",
                "vector_data",
                "plain_titles",
            ],
        }
    }
}

#[derive(Debug, Serialize)]
struct PhoenixSpan {
    trace_id: String,
    span_id: String,
    parent_span_id: Option<String>,
    name: String,
    openinference_span_kind: &'static str,
    component: String,
    operation: String,
    outcome: String,
    reason_code: Option<String>,
    provider_id: Option<String>,
    model_id: Option<String>,
    template_version: Option<String>,
    attributes: BTreeMap<String, serde_json::Value>,
}

impl PhoenixSpan {
    fn from_record(record: &TraceRecord) -> Result<Self, PhoenixExportError> {
        let component = enum_string(&record.span.component)?;
        let operation = enum_string(&record.span.operation)?;
        let outcome = enum_string(&record.span.outcome)?;
        let name = format!("{component}.{operation}");
        let attributes = span_attributes(record, &component, &operation, &outcome)?;
        Ok(Self {
            trace_id: record.trace.trace_id.clone(),
            span_id: record.trace.span_id.clone(),
            parent_span_id: record.trace.parent_span_id.clone(),
            name,
            openinference_span_kind: "CHAIN",
            component,
            operation,
            outcome,
            reason_code: record.span.reason_code.clone(),
            provider_id: record.span.provider_id.clone(),
            model_id: record.span.model_id.clone(),
            template_version: record.span.template_version.clone(),
            attributes,
        })
    }
}

fn span_attributes(
    record: &TraceRecord,
    component: &str,
    operation: &str,
    outcome: &str,
) -> Result<BTreeMap<String, serde_json::Value>, PhoenixExportError> {
    let mut attributes = BTreeMap::new();
    attributes.insert("openinference.span.kind".to_owned(), json!("CHAIN"));
    attributes.insert("morrow.component".to_owned(), json!(component));
    attributes.insert("morrow.operation".to_owned(), json!(operation));
    attributes.insert("morrow.outcome".to_owned(), json!(outcome));
    attributes.insert(
        "morrow.trace_schema_version".to_owned(),
        json!(enum_string(&record.schema_version)?),
    );
    attributes.insert(
        "morrow.privacy.tier".to_owned(),
        json!(enum_string(&record.span.privacy_tier)?),
    );
    attributes.insert(
        "morrow.privacy.title_status".to_owned(),
        json!(record.span.title_status.as_deref().unwrap_or("absent")),
    );
    attributes.insert(
        "morrow.privacy.title_hash_present".to_owned(),
        json!(record.span.title_hash.is_some()),
    );
    attributes.insert(
        "morrow.privacy.dropped_category_count".to_owned(),
        json!(PhoenixPrivacy::default().dropped_categories.len()),
    );
    if let Some(reason_code) = &record.span.reason_code {
        attributes.insert("morrow.reason_code".to_owned(), json!(reason_code));
    }
    if let Some(decision) = &record.span.decision {
        attributes.insert("morrow.decision".to_owned(), json!(enum_string(decision)?));
    }
    if let Some(provider_id) = &record.span.provider_id {
        attributes.insert("llm.provider".to_owned(), json!(provider_id));
    }
    if let Some(model_id) = &record.span.model_id {
        attributes.insert("llm.model_name".to_owned(), json!(model_id));
    }
    if let Some(template_version) = &record.span.template_version {
        attributes.insert(
            "morrow.template.version".to_owned(),
            json!(template_version),
        );
    }
    if let Some(confidence_millis) = record.span.confidence_millis {
        attributes.insert(
            "morrow.confidence_millis".to_owned(),
            json!(confidence_millis),
        );
    }
    Ok(attributes)
}

fn enum_string<T: Serialize>(value: &T) -> Result<String, PhoenixExportError> {
    let value = serde_json::to_value(value)?;
    let Some(value) = value.as_str() else {
        return Err(PhoenixExportError::EnumString);
    };
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests;
