use std::error::Error;
use std::fmt::{Display, Formatter};

use morrow_storage::{CandidateKind, FeedbackPrivacyTier};

pub(crate) const SCORE_TOTAL: u16 = 100;
pub(crate) const RAW_PRIVACY_CANARY: &str = "PHASE5_RAW_MESSAGE_PROVIDER_CANARY_DO_NOT_STORE";

pub(super) const MAX_PLACEHOLDER_LEN: usize = 64;
pub(super) const FORBIDDEN_FIELD_NAMES: [&str; 14] = [
    "raw_text",
    "raw_message",
    "raw_messages_body",
    "provider_payload",
    "provider_response",
    "provider_json",
    "chat_guid",
    "message_guid",
    "native_id",
    "native_identifier",
    "real_title",
    "unredacted_title",
    "prompt",
    "response",
];
pub(super) const FORBIDDEN_TEXT_MARKERS: [&str; 7] = [
    RAW_PRIVACY_CANARY,
    "messages://",
    "chat-",
    "msg-",
    "provider_response",
    "native:",
    "BEGIN PROVIDER JSON",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProposalKindExpectation {
    CalendarEvent,
    TaskReminder,
    NoProposal,
}

impl ProposalKindExpectation {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::CalendarEvent => "calendar_event",
            Self::TaskReminder => "task_reminder",
            Self::NoProposal => "no_proposal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExpectedOutcome {
    ProposalAccepted,
    ProposalRejected,
    ProposalEditedBeforeApproval,
    QuietLowConfidence,
    NonTargetPreserved,
    ReplayIdempotent,
    PrivacyCanaryRejected,
}

impl ExpectedOutcome {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ProposalAccepted => "proposal_accepted",
            Self::ProposalRejected => "proposal_rejected",
            Self::ProposalEditedBeforeApproval => "proposal_edited_before_approval",
            Self::QuietLowConfidence => "quiet_low_confidence",
            Self::NonTargetPreserved => "non_target_preserved",
            Self::ReplayIdempotent => "replay_idempotent",
            Self::PrivacyCanaryRejected => "privacy_canary_rejected",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AllowedSideEffect {
    CreateCalendarProposal,
    CreateReminderProposal,
    RemoveProposedExternal,
    RecordFeedbackOnly,
    PreserveNonTargetExternal,
    RejectBeforeMutation,
}

impl AllowedSideEffect {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::CreateCalendarProposal => "create_calendar_proposal",
            Self::CreateReminderProposal => "create_reminder_proposal",
            Self::RemoveProposedExternal => "remove_proposed_external",
            Self::RecordFeedbackOnly => "record_feedback_only",
            Self::PreserveNonTargetExternal => "preserve_non_target_external",
            Self::RejectBeforeMutation => "reject_before_mutation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CleanupAction {
    DeleteProposedExternal,
    NoExternalCreated,
    PreserveNonTargetOnly,
}

impl CleanupAction {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::DeleteProposedExternal => "delete_proposed_external",
            Self::NoExternalCreated => "no_external_created",
            Self::PreserveNonTargetOnly => "preserve_non_target_only",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExpectedLabel {
    pub(crate) label_type: &'static str,
    pub(crate) label_value: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExpectedTraceOperation {
    pub(crate) component: &'static str,
    pub(crate) operation: &'static str,
    pub(crate) outcome: &'static str,
    pub(crate) privacy_tier: FeedbackPrivacyTier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CleanupExpectation {
    pub(crate) action: CleanupAction,
    pub(crate) non_target_fixture_id: Option<&'static str>,
    pub(crate) live_surface_used: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScoreWeight {
    pub(crate) category: &'static str,
    pub(crate) weight: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TrajectoryCase {
    pub(crate) trajectory_case_id: &'static str,
    pub(crate) family: &'static str,
    pub(crate) source_fixture_id: &'static str,
    pub(crate) candidate_fixture_id: &'static str,
    pub(crate) proposal_fixture_id: &'static str,
    pub(crate) bounded_excerpt_placeholder: &'static str,
    pub(crate) expected_candidate_kind: CandidateKind,
    pub(crate) expected_proposal_kind: ProposalKindExpectation,
    pub(crate) expected_outcomes: &'static [ExpectedOutcome],
    pub(crate) expected_labels: &'static [ExpectedLabel],
    pub(crate) expected_trace_operations: &'static [ExpectedTraceOperation],
    pub(crate) allowed_side_effects: &'static [AllowedSideEffect],
    pub(crate) cleanup: Option<CleanupExpectation>,
    pub(crate) score_weights: &'static [ScoreWeight],
    pub(crate) explicit_score_total: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SchemaValidationError {
    reason: String,
}

impl SchemaValidationError {
    pub(crate) fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl Display for SchemaValidationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.reason)
    }
}

impl Error for SchemaValidationError {}
