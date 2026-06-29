use crate::trace::{TraceComponent, TraceDecision, TraceOperation, TraceOutcome, TracePrivacyTier};

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct LangfusePayload {
    pub schema_version: &'static str,
    pub source: &'static str,
    pub ownership: &'static str,
    pub traces: Vec<LangfuseTrace>,
    pub export: LangfuseExportReceipt,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct LangfuseTrace {
    pub id: String,
    pub name: String,
    pub timestamp: String,
    pub metadata: LangfuseTraceMetadata,
    pub observations: Vec<LangfuseObservation>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct LangfuseTraceMetadata {
    pub source_trace_id: String,
    pub source: &'static str,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct LangfuseObservation {
    pub id: String,
    pub trace_id: String,
    pub parent_observation_id: Option<String>,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub start_time: String,
    pub end_time: Option<String>,
    pub input: MaskedValue,
    pub output: MaskedValue,
    pub metadata: LangfuseObservationMetadata,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct MaskedValue {
    pub status: &'static str,
    pub policy: &'static str,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct LangfuseObservationMetadata {
    pub component: TraceComponent,
    pub operation: TraceOperation,
    pub decision: Option<TraceDecision>,
    pub outcome: TraceOutcome,
    pub reason_code: Option<String>,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub confidence_millis: Option<u16>,
    pub title_status: Option<String>,
    pub privacy_tier: TracePrivacyTier,
    pub classifier_stage: Option<String>,
    pub router_stage: Option<String>,
    pub ood_score_millis: Option<u16>,
    pub replay_run_id: Option<String>,
    pub dropped: DroppedMetadata,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct DroppedMetadata {
    pub policy: &'static str,
    pub categories: Vec<DroppedCategory>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DroppedCategory {
    UserContent,
    VendorPayload,
    VectorValues,
    NativeIdentifier,
    TitleContent,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct LangfuseExportReceipt {
    pub records_read: usize,
    pub skipped_corrupt_lines: usize,
    pub masking_policy: &'static str,
    pub dropped_categories: Vec<DroppedCategory>,
    pub backend: LangfuseBackendReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LangfuseBackendConfig {
    pub endpoint: Option<String>,
    pub public_key: Option<String>,
    pub secret_key: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct LangfuseBackendReceipt {
    pub status: LangfuseBackendStatus,
    pub network_attempted: bool,
    pub endpoint_configured: bool,
    pub credentials_configured: bool,
    pub detail: &'static str,
}

#[derive(Debug, Clone, Copy, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LangfuseBackendStatus {
    NotConfigured,
    MissingCredentials,
    OfflineOnly,
}
