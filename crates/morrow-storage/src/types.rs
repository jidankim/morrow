use crate::CandidateId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateKind {
    CalendarEvent,
    TaskReminder,
    EventUpdate,
    EventReschedule,
    EventCancellation,
    ReminderUpdate,
    ReminderReschedule,
    ReminderCancellation,
}

impl CandidateKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CalendarEvent => "calendar_event",
            Self::TaskReminder => "task_reminder",
            Self::EventUpdate => "event_update",
            Self::EventReschedule => "event_reschedule",
            Self::EventCancellation => "event_cancellation",
            Self::ReminderUpdate => "reminder_update",
            Self::ReminderReschedule => "reminder_reschedule",
            Self::ReminderCancellation => "reminder_cancellation",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, crate::StorageError> {
        match raw {
            "calendar_event" => Ok(Self::CalendarEvent),
            "task_reminder" => Ok(Self::TaskReminder),
            "event_update" => Ok(Self::EventUpdate),
            "event_reschedule" => Ok(Self::EventReschedule),
            "event_cancellation" => Ok(Self::EventCancellation),
            "reminder_update" => Ok(Self::ReminderUpdate),
            "reminder_reschedule" => Ok(Self::ReminderReschedule),
            "reminder_cancellation" => Ok(Self::ReminderCancellation),
            other => Err(crate::StorageError::InvalidInput {
                field: "candidate_kind",
                reason: format!("unknown kind {other}"),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateState {
    Queued,
    CreatingExternal,
    Visible,
    Approved,
    Completed,
    Rejected,
    Expired,
    Suppressed,
    Unknown,
    Failed,
}

impl CandidateState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::CreatingExternal => "creating_external",
            Self::Visible => "visible",
            Self::Approved => "approved",
            Self::Completed => "completed",
            Self::Rejected => "rejected",
            Self::Expired => "expired",
            Self::Suppressed => "suppressed",
            Self::Unknown => "unknown",
            Self::Failed => "failed",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, crate::StorageError> {
        match raw {
            "queued" => Ok(Self::Queued),
            "creating_external" => Ok(Self::CreatingExternal),
            "visible" => Ok(Self::Visible),
            "approved" => Ok(Self::Approved),
            "completed" => Ok(Self::Completed),
            "rejected" => Ok(Self::Rejected),
            "expired" => Ok(Self::Expired),
            "suppressed" => Ok(Self::Suppressed),
            "unknown" => Ok(Self::Unknown),
            "failed" => Ok(Self::Failed),
            other => Err(crate::StorageError::InvalidInput {
                field: "candidate_state",
                reason: format!("unknown state {other}"),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalSource {
    Calendar,
    Reminders,
}

impl ExternalSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Calendar => "calendar",
            Self::Reminders => "reminders",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayStream {
    CalendarProposals,
}

impl ReplayStream {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CalendarProposals => "calendar_proposals",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CandidateDraft {
    pub kind: CandidateKind,
    pub chat_guid: String,
    pub anchor_message_guid: String,
    pub title: String,
    pub confidence_millis: i64,
    pub normalized_time: String,
    pub evidence_excerpt: String,
    pub observed_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateFeedbackContext {
    pub candidate_id: CandidateId,
    pub chat_guid: String,
    pub anchor_message_guid: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateLifecycleReadback {
    pub candidate_id: CandidateId,
    pub kind: CandidateKind,
    pub state: CandidateState,
    pub current_reason: String,
}

#[derive(Debug, Clone)]
pub struct QuietLogDraft {
    pub chat_guid: String,
    pub anchor_message_guid: String,
    pub reason: String,
    pub excerpt: String,
    pub provider_diagnostic: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct ExternalObjectMapping {
    pub candidate_id: CandidateId,
    pub source: ExternalSource,
    pub external_object_id: String,
    pub external_source_id: String,
    pub mapped_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarProposalPayload {
    pub candidate_id: CandidateId,
    pub kind: CandidateKind,
    pub normalized_time: String,
    pub title: String,
    pub source_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReminderProposalPayload {
    pub candidate_id: CandidateId,
    pub kind: CandidateKind,
    pub normalized_time: String,
    pub title: String,
    pub source_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEntry {
    pub from_state: CandidateState,
    pub to_state: CandidateState,
    pub reason: String,
}

pub const PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE: &str = "Messages event candidate";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderRouteSourceExcerptPolicy {
    Include,
    Hide,
}

impl ProviderRouteSourceExcerptPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Include => "include",
            Self::Hide => "hide",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, crate::StorageError> {
        match raw {
            "include" => Ok(Self::Include),
            "hide" => Ok(Self::Hide),
            other => Err(crate::StorageError::InvalidInput {
                field: "source_excerpt_policy",
                reason: format!("unknown source excerpt policy {other}"),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderRouteOutcomeKind {
    Candidate,
    Quiet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderRouteRecordStatus {
    Recorded,
    SkippedProviderUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRouteCandidate {
    pub kind: CandidateKind,
    pub title: String,
    pub confidence_millis: i64,
    pub normalized_time: String,
    pub evidence_excerpt: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderRouteOutcome {
    Candidate(ProviderRouteCandidate),
    Quiet { quiet_reason: String },
    ProviderUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRouteOutcomeDraft {
    pub route_fingerprint: String,
    pub provider_route_contract_version: String,
    pub provider_candidate_schema_version: String,
    pub evidence_payload_hash: String,
    pub provider_id: String,
    pub model_id: String,
    pub prompt_version: String,
    pub source_excerpt_policy: ProviderRouteSourceExcerptPolicy,
    pub reference_observed: String,
    pub reference_timezone: String,
    pub threshold_millis: i64,
    pub parser_route: String,
    pub outcome: ProviderRouteOutcome,
    pub observed_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderRouteStoredOutcome {
    Candidate(ProviderRouteCandidate),
    Quiet { quiet_reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRouteLedgerRow {
    pub route_fingerprint: String,
    pub provider_route_contract_version: String,
    pub provider_candidate_schema_version: String,
    pub evidence_payload_hash: String,
    pub provider_id: String,
    pub model_id: String,
    pub prompt_version: String,
    pub source_excerpt_policy: ProviderRouteSourceExcerptPolicy,
    pub reference_observed: String,
    pub reference_timezone: String,
    pub threshold_millis: i64,
    pub parser_route: String,
    pub outcome: ProviderRouteStoredOutcome,
    pub created_at: i64,
    pub updated_at: i64,
}

impl ProviderRouteLedgerRow {
    pub const fn outcome_kind(&self) -> ProviderRouteOutcomeKind {
        match &self.outcome {
            ProviderRouteStoredOutcome::Candidate(_) => ProviderRouteOutcomeKind::Candidate,
            ProviderRouteStoredOutcome::Quiet { .. } => ProviderRouteOutcomeKind::Quiet,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivacySummary {
    pub table_count: usize,
    pub full_message_body_columns: usize,
    pub max_excerpt_len: usize,
}
