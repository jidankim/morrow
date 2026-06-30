use crate::feedback_eval::{
    DetectionRouteLabel, FeedbackLabelType, FeedbackLabelValue, FieldQualityLabel,
    ProposalOutcomeLabel, SystemOutcomeLabel,
};
use crate::StorageError;

impl ProposalOutcomeLabel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::RejectedObserved => "rejected_observed",
            Self::PendingEdited => "pending_edited",
            Self::Unknown => "unknown",
        }
    }
}

impl DetectionRouteLabel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DeterministicCandidate => "deterministic_candidate",
            Self::ProviderCandidate => "provider_candidate",
            Self::QuietStop => "quiet_stop",
            Self::ProviderRejected => "provider_rejected",
            Self::ProviderUnavailable => "provider_unavailable",
        }
    }
}

impl FieldQualityLabel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::TitleEdited => "title_edited",
            Self::TimeEdited => "time_edited",
            Self::KindEdited => "kind_edited",
        }
    }
}

impl SystemOutcomeLabel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::FailedExternalCreation => "failed_external_creation",
            Self::FailedProvider => "failed_provider",
            Self::FailedValidation => "failed_validation",
        }
    }
}

impl FeedbackLabelValue {
    pub const fn label_type(self) -> FeedbackLabelType {
        match self {
            Self::ProposalOutcome(_) => FeedbackLabelType::ProposalOutcome,
            Self::DetectionRoute(_) => FeedbackLabelType::DetectionRoute,
            Self::FieldQuality(_) => FeedbackLabelType::FieldQuality,
            Self::SystemOutcome(_) => FeedbackLabelType::SystemOutcome,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProposalOutcome(value) => value.as_str(),
            Self::DetectionRoute(value) => value.as_str(),
            Self::FieldQuality(value) => value.as_str(),
            Self::SystemOutcome(value) => value.as_str(),
        }
    }

    pub fn parse(
        label_type: FeedbackLabelType,
        raw: &str,
        field: &'static str,
    ) -> Result<Self, StorageError> {
        match label_type {
            FeedbackLabelType::ProposalOutcome => proposal_outcome(raw, field),
            FeedbackLabelType::DetectionRoute => detection_route(raw, field),
            FeedbackLabelType::FieldQuality => field_quality(raw, field),
            FeedbackLabelType::SystemOutcome => system_outcome(raw, field),
        }
    }
}

fn proposal_outcome(raw: &str, field: &'static str) -> Result<FeedbackLabelValue, StorageError> {
    match raw {
        "accepted" => Ok(FeedbackLabelValue::ProposalOutcome(
            ProposalOutcomeLabel::Accepted,
        )),
        "rejected_observed" => Ok(FeedbackLabelValue::ProposalOutcome(
            ProposalOutcomeLabel::RejectedObserved,
        )),
        "pending_edited" => Ok(FeedbackLabelValue::ProposalOutcome(
            ProposalOutcomeLabel::PendingEdited,
        )),
        "unknown" => Ok(FeedbackLabelValue::ProposalOutcome(
            ProposalOutcomeLabel::Unknown,
        )),
        other => invalid(
            field,
            &format!("invalid proposal_outcome label value {other}"),
        ),
    }
}

fn detection_route(raw: &str, field: &'static str) -> Result<FeedbackLabelValue, StorageError> {
    match raw {
        "deterministic_candidate" => Ok(FeedbackLabelValue::DetectionRoute(
            DetectionRouteLabel::DeterministicCandidate,
        )),
        "provider_candidate" => Ok(FeedbackLabelValue::DetectionRoute(
            DetectionRouteLabel::ProviderCandidate,
        )),
        "quiet_stop" => Ok(FeedbackLabelValue::DetectionRoute(
            DetectionRouteLabel::QuietStop,
        )),
        "provider_rejected" => Ok(FeedbackLabelValue::DetectionRoute(
            DetectionRouteLabel::ProviderRejected,
        )),
        "provider_unavailable" => Ok(FeedbackLabelValue::DetectionRoute(
            DetectionRouteLabel::ProviderUnavailable,
        )),
        other => invalid(
            field,
            &format!("invalid detection_route label value {other}"),
        ),
    }
}

fn field_quality(raw: &str, field: &'static str) -> Result<FeedbackLabelValue, StorageError> {
    match raw {
        "unknown" => Ok(FeedbackLabelValue::FieldQuality(FieldQualityLabel::Unknown)),
        "title_edited" => Ok(FeedbackLabelValue::FieldQuality(
            FieldQualityLabel::TitleEdited,
        )),
        "time_edited" => Ok(FeedbackLabelValue::FieldQuality(
            FieldQualityLabel::TimeEdited,
        )),
        "kind_edited" => Ok(FeedbackLabelValue::FieldQuality(
            FieldQualityLabel::KindEdited,
        )),
        other => invalid(field, &format!("invalid field_quality label value {other}")),
    }
}

fn system_outcome(raw: &str, field: &'static str) -> Result<FeedbackLabelValue, StorageError> {
    match raw {
        "ok" => Ok(FeedbackLabelValue::SystemOutcome(SystemOutcomeLabel::Ok)),
        "failed_external_creation" => Ok(FeedbackLabelValue::SystemOutcome(
            SystemOutcomeLabel::FailedExternalCreation,
        )),
        "failed_provider" => Ok(FeedbackLabelValue::SystemOutcome(
            SystemOutcomeLabel::FailedProvider,
        )),
        "failed_validation" => Ok(FeedbackLabelValue::SystemOutcome(
            SystemOutcomeLabel::FailedValidation,
        )),
        other => invalid(
            field,
            &format!("invalid system_outcome label value {other}"),
        ),
    }
}

fn invalid<T>(field: &'static str, reason: &str) -> Result<T, StorageError> {
    Err(StorageError::InvalidInput {
        field,
        reason: reason.to_owned(),
    })
}
