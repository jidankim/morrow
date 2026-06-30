use crate::CandidateId;

pub const FEEDBACK_EVAL_SCHEMA_VERSION: i64 = 1;
pub const REDACTED_SOURCE_EXCERPT: &str = "Source excerpt hidden by settings.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackSubjectType {
    Candidate,
    QuietLog,
    EvalCase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackLabelSource {
    Lifecycle,
    QuietLog,
    Provider,
    EvalRunner,
    ManualAlpha,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackSourceExcerptPolicy {
    Include,
    Hide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackPrivacyTier {
    InternalMetadata,
    HashedIdentifier,
    LocalPrivate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackEventType {
    CandidateVisible,
    ApprovedByMove,
    ApprovedByCopy,
    RejectedByDelete,
    PendingEdited,
    ProposedReminderCompletedResolved,
    UnknownDisappearance,
    ExternalCreationFailed,
    DeterministicStopUnsupportedBroadContent,
    DeterministicStopInvalidEvidence,
    DeterministicStopNoSchedulingSignal,
    DeterministicStopPastOrInvalidTime,
    ConfidenceBelowThreshold,
    ProviderUnavailable,
    ProviderInvalidJson,
    ProviderSchemaRejected,
    ProviderHallucinatedEvidence,
    ParserProviderTimeConflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackLabelType {
    ProposalOutcome,
    DetectionRoute,
    FieldQuality,
    SystemOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalOutcomeLabel {
    Accepted,
    RejectedObserved,
    PendingEdited,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionRouteLabel {
    DeterministicCandidate,
    ProviderCandidate,
    QuietStop,
    ProviderRejected,
    ProviderUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldQualityLabel {
    Unknown,
    TitleEdited,
    TimeEdited,
    KindEdited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemOutcomeLabel {
    Ok,
    FailedExternalCreation,
    FailedProvider,
    FailedValidation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackLabelValue {
    ProposalOutcome(ProposalOutcomeLabel),
    DetectionRoute(DetectionRouteLabel),
    FieldQuality(FieldQualityLabel),
    SystemOutcome(SystemOutcomeLabel),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalRunStatus {
    Passed,
    NeedsReview,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalResultOutcome {
    Match,
    Mismatch,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticsTraceLinkage {
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub parent_span_id: Option<String>,
    pub chat_hash: Option<String>,
    pub message_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackRecordMeta {
    pub schema_version: i64,
    pub subject_type: FeedbackSubjectType,
    pub subject_id: String,
    pub candidate_id: Option<CandidateId>,
    pub chat_guid: String,
    pub anchor_message_guid: String,
    pub diagnostics: DiagnosticsTraceLinkage,
    pub provider_model_prompt_version_id: Option<i64>,
    pub source_excerpt_policy: FeedbackSourceExcerptPolicy,
    pub label_source: FeedbackLabelSource,
    pub privacy_tier: FeedbackPrivacyTier,
    pub privacy_metadata_json: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackEvent {
    pub id: Option<i64>,
    pub event_key: String,
    pub event_type: FeedbackEventType,
    pub meta: FeedbackRecordMeta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub id: Option<i64>,
    pub label_key: String,
    pub label_value: FeedbackLabelValue,
    pub meta: FeedbackRecordMeta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureSnapshot {
    pub id: Option<i64>,
    pub snapshot_key: String,
    pub meta: FeedbackRecordMeta,
    pub route: Option<String>,
    pub reason_code: Option<String>,
    pub confidence_millis: Option<i64>,
    pub participant_count: Option<i64>,
    pub tapback_signal: Option<String>,
    pub sender_signal_available: bool,
    pub context_window_available: bool,
    pub excerpt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalRun {
    pub id: Option<i64>,
    pub run_key: String,
    pub status: EvalRunStatus,
    pub schema_version: i64,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub report_path: Option<String>,
    pub cases_evaluated: i64,
    pub cases_skipped: i64,
    pub skip_reasons_json: String,
    pub approval_kept_rate_millis: i64,
    pub observed_rejection_rate_millis: i64,
    pub quiet_log_count: i64,
    pub provider_failure_count: i64,
    pub accepted_visible_ratio_millis: i64,
    pub confusion_counts_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackEvalCounts {
    pub label_count: i64,
    pub feature_snapshot_count: i64,
    pub latest_eval_status: Option<EvalRunStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalResult {
    pub id: Option<i64>,
    pub result_key: String,
    pub eval_run_id: i64,
    pub snapshot_key: String,
    pub label_key: String,
    pub expected_label_type: FeedbackLabelType,
    pub expected_label_value: String,
    pub actual_label_type: Option<FeedbackLabelType>,
    pub actual_label_value: Option<String>,
    pub outcome: EvalResultOutcome,
    pub skip_reason: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalCase {
    pub snapshot: FeatureSnapshot,
    pub label: Label,
}
