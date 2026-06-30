use crate::feedback_eval::{
    EvalResultOutcome, EvalRunStatus, FeedbackEventType, FeedbackLabelSource, FeedbackLabelType,
    FeedbackPrivacyTier, FeedbackSourceExcerptPolicy, FeedbackSubjectType,
};
use crate::StorageError;

impl FeedbackSubjectType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::QuietLog => "quiet_log",
            Self::EvalCase => "eval_case",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, StorageError> {
        match raw {
            "candidate" => Ok(Self::Candidate),
            "quiet_log" => Ok(Self::QuietLog),
            "eval_case" => Ok(Self::EvalCase),
            other => invalid("subject_type", &format!("unknown subject type {other}")),
        }
    }
}

impl FeedbackLabelSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lifecycle => "lifecycle",
            Self::QuietLog => "quiet_log",
            Self::Provider => "provider",
            Self::EvalRunner => "eval_runner",
            Self::ManualAlpha => "manual_alpha",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, StorageError> {
        match raw {
            "lifecycle" => Ok(Self::Lifecycle),
            "quiet_log" => Ok(Self::QuietLog),
            "provider" => Ok(Self::Provider),
            "eval_runner" => Ok(Self::EvalRunner),
            "manual_alpha" => Ok(Self::ManualAlpha),
            other => invalid("label_source", &format!("unknown label source {other}")),
        }
    }
}

impl FeedbackSourceExcerptPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Include => "include",
            Self::Hide => "hide",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, StorageError> {
        match raw {
            "include" => Ok(Self::Include),
            "hide" => Ok(Self::Hide),
            other => invalid(
                "source_excerpt_policy",
                &format!("unknown source excerpt policy {other}"),
            ),
        }
    }
}

impl FeedbackPrivacyTier {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InternalMetadata => "internal_metadata",
            Self::HashedIdentifier => "hashed_identifier",
            Self::LocalPrivate => "local_private",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, StorageError> {
        match raw {
            "internal_metadata" => Ok(Self::InternalMetadata),
            "hashed_identifier" => Ok(Self::HashedIdentifier),
            "local_private" => Ok(Self::LocalPrivate),
            other => invalid("privacy_tier", &format!("unknown privacy tier {other}")),
        }
    }
}

impl FeedbackEventType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CandidateVisible => "candidate_visible",
            Self::ApprovedByMove => "approved_by_move",
            Self::ApprovedByCopy => "approved_by_copy",
            Self::RejectedByDelete => "rejected_by_delete",
            Self::PendingEdited => "pending_edited",
            Self::ProposedReminderCompletedResolved => "proposed_reminder_completed_resolved",
            Self::UnknownDisappearance => "unknown_disappearance",
            Self::ExternalCreationFailed => "external_creation_failed",
            Self::DeterministicStopUnsupportedBroadContent => {
                "deterministic_stop:unsupported_broad_content"
            }
            Self::DeterministicStopInvalidEvidence => "deterministic_stop:invalid_evidence",
            Self::DeterministicStopNoSchedulingSignal => "deterministic_stop:no_scheduling_signal",
            Self::DeterministicStopPastOrInvalidTime => "deterministic_stop:past_or_invalid_time",
            Self::ConfidenceBelowThreshold => "confidence_below_threshold",
            Self::ProviderUnavailable => "provider_unavailable",
            Self::ProviderInvalidJson => "provider_invalid_json",
            Self::ProviderSchemaRejected => "provider_schema_rejected",
            Self::ProviderHallucinatedEvidence => "provider_hallucinated_evidence",
            Self::ParserProviderTimeConflict => "parser_provider_time_conflict",
        }
    }
}

impl FeedbackLabelType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProposalOutcome => "proposal_outcome",
            Self::DetectionRoute => "detection_route",
            Self::FieldQuality => "field_quality",
            Self::SystemOutcome => "system_outcome",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, StorageError> {
        match raw {
            "proposal_outcome" => Ok(Self::ProposalOutcome),
            "detection_route" => Ok(Self::DetectionRoute),
            "field_quality" => Ok(Self::FieldQuality),
            "system_outcome" => Ok(Self::SystemOutcome),
            other => invalid("label_type", &format!("unknown label type {other}")),
        }
    }
}

impl EvalRunStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::NeedsReview => "needs_review",
            Self::Failed => "failed",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, StorageError> {
        match raw {
            "passed" => Ok(Self::Passed),
            "needs_review" => Ok(Self::NeedsReview),
            "failed" => Ok(Self::Failed),
            other => invalid(
                "eval_run_status",
                &format!("unknown eval run status {other}"),
            ),
        }
    }
}

impl EvalResultOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Match => "match",
            Self::Mismatch => "mismatch",
            Self::Skipped => "skipped",
        }
    }
}

fn invalid<T>(field: &'static str, reason: &str) -> Result<T, StorageError> {
    Err(StorageError::InvalidInput {
        field,
        reason: reason.to_owned(),
    })
}
