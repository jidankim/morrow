use std::io::Read as _;

use sha2::{Digest, Sha256};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TraceRecord {
    pub schema_version: TraceSchemaVersion,
    pub trace: TraceIds,
    pub span: TraceSpan,
}

impl TraceRecord {
    pub fn sample_v1() -> Self {
        Self {
            schema_version: TraceSchemaVersion::V1,
            trace: TraceIds::sample_v1(),
            span: TraceSpan::sample_v1(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TraceSpan {
    pub component: TraceComponent,
    pub operation: TraceOperation,
    pub decision: Option<TraceDecision>,
    pub outcome: TraceOutcome,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub template_version: Option<String>,
    pub reason_code: Option<String>,
    pub confidence_millis: Option<u16>,
    pub title_hash: Option<String>,
    pub title_status: Option<String>,
    pub privacy_tier: TracePrivacyTier,
    pub classifier_stage: Option<String>,
    pub router_stage: Option<String>,
    pub ood_score_millis: Option<u16>,
    pub replay_run_id: Option<String>,
}

impl TraceSpan {
    pub fn sample_v1() -> Self {
        Self {
            component: TraceComponent::Lifecycle,
            operation: TraceOperation::PollEmpty,
            decision: Some(TraceDecision::LifecycleUpdated),
            outcome: TraceOutcome::Noop,
            started_at: "2026-06-28T05:00:00Z".to_owned(),
            ended_at: Some("2026-06-28T05:00:00Z".to_owned()),
            provider_id: None,
            model_id: None,
            template_version: None,
            reason_code: Some("no_messages_ready".to_owned()),
            confidence_millis: None,
            title_hash: Some(hash_identifier("team sync")),
            title_status: Some("hashed".to_owned()),
            privacy_tier: TracePrivacyTier::HashedIdentifier,
            classifier_stage: Some("reserved_classifier".to_owned()),
            router_stage: Some("reserved_router".to_owned()),
            ood_score_millis: Some(0),
            replay_run_id: Some("replay-local-fixture".to_owned()),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TraceIds {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub chat_hash: Option<String>,
    pub message_hash: Option<String>,
}

impl TraceIds {
    pub fn random(
        chat_identifier: Option<&str>,
        message_identifier: Option<&str>,
    ) -> Result<Self, OpaqueIdError> {
        Ok(Self {
            trace_id: random_opaque_id("trace")?,
            span_id: random_opaque_id("span")?,
            parent_span_id: None,
            chat_hash: chat_identifier.map(hash_identifier),
            message_hash: message_identifier.map(hash_identifier),
        })
    }

    pub fn child(
        parent: &TraceIds,
        message_identifier: Option<&str>,
    ) -> Result<Self, OpaqueIdError> {
        Ok(Self {
            trace_id: parent.trace_id.clone(),
            span_id: random_opaque_id("span")?,
            parent_span_id: Some(parent.span_id.clone()),
            chat_hash: parent.chat_hash.clone(),
            message_hash: message_identifier.map(hash_identifier),
        })
    }

    fn sample_v1() -> Self {
        Self {
            trace_id: "trace_018fda8a98bf4cdba33a6f9d42180d6d".to_owned(),
            span_id: "span_52efdebab8574a5fa6f290831a786e86".to_owned(),
            parent_span_id: None,
            chat_hash: Some(hash_identifier("chat-guid-fixture")),
            message_hash: Some(hash_identifier("message-guid-fixture")),
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OpaqueIdError {
    #[error("opaque trace identifier generation failed")]
    Generation,
    #[error("opaque trace identifier entropy source failed")]
    Entropy,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TraceComponent {
    Detection,
    Parser,
    Provider,
    Schema,
    Threshold,
    Outcome,
    Storage,
    Classifier,
    Router,
    Ood,
    Lifecycle,
    Correction,
    Calendar,
    Replay,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TraceOperation {
    ParserDecision,
    ProviderRoute,
    ProviderResult,
    SchemaValidation,
    ThresholdDecision,
    OutcomeMaterialized,
    ClassifierScore,
    RouterDecision,
    OodCheck,
    PollEmpty,
    CursorAdvanced,
    UserCorrection,
    CandidateSuperseded,
    CandidateRescheduled,
    CandidateCancelled,
    CalendarDryRun,
    CalendarCommitIdempotency,
    ReplayRun,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TraceDecision {
    Stop,
    Candidate,
    ProviderRoute,
    ProviderUnavailable,
    SchemaRejected,
    ConfidenceAccepted,
    ConfidenceRejected,
    ClassifierAccepted,
    ClassifierRejected,
    RouterAccepted,
    RouterFallback,
    OodAccepted,
    OodRejected,
    UserCorrected,
    LifecycleUpdated,
    ReplayCompared,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TraceOutcome {
    CandidateCreated,
    QuietLogged,
    Noop,
    Rejected,
    Superseded,
    Rescheduled,
    Cancelled,
    DryRun,
    CommitIdempotent,
    ReplayRecorded,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TracePrivacyTier {
    Public,
    InternalMetadata,
    HashedIdentifier,
    LocalPrivate,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TraceSchemaVersion {
    V1,
}

pub trait TraceRecorder {
    fn record(&self, record: &TraceRecord) -> Result<(), TraceRecorderError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoopTraceRecorder;

impl TraceRecorder for NoopTraceRecorder {
    fn record(&self, _record: &TraceRecord) -> Result<(), TraceRecorderError> {
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TraceRecorderError {
    #[error("trace recorder unavailable")]
    Unavailable,
}

fn random_opaque_id(prefix: &str) -> Result<String, OpaqueIdError> {
    let mut bytes = [0_u8; 16];
    let mut entropy = std::fs::File::open("/dev/urandom").map_err(|_| OpaqueIdError::Entropy)?;
    entropy
        .read_exact(&mut bytes)
        .map_err(|_| OpaqueIdError::Generation)?;
    Ok(format!("{prefix}_{}", hex_bytes(&bytes)))
}

fn hash_identifier(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    format!("sha256:{digest:x}")
}

fn hex_bytes(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}
