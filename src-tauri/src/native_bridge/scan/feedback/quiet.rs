use morrow_storage::{
    DetectionRouteLabel, FeedbackEventType, FeedbackLabelValue, ProposalOutcomeLabel,
    QuietLogDraft, SystemOutcomeLabel,
};

use crate::native_bridge::scan::ScanSelectedChatsError;

pub(super) struct QuietMapping {
    pub(super) event_type: FeedbackEventType,
    pub(super) snapshot_route: &'static str,
    pub(super) labels: &'static [FeedbackLabelValue],
}

const QUIET_STOP_LABELS: &[FeedbackLabelValue] = &[FeedbackLabelValue::DetectionRoute(
    DetectionRouteLabel::QuietStop,
)];
const PROVIDER_REJECTED_UNKNOWN_LABELS: &[FeedbackLabelValue] = &[
    FeedbackLabelValue::DetectionRoute(DetectionRouteLabel::ProviderRejected),
    FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::Unknown),
];
const PROVIDER_UNAVAILABLE_LABELS: &[FeedbackLabelValue] = &[
    FeedbackLabelValue::DetectionRoute(DetectionRouteLabel::ProviderUnavailable),
    FeedbackLabelValue::SystemOutcome(SystemOutcomeLabel::FailedProvider),
];
const PROVIDER_VALIDATION_LABELS: &[FeedbackLabelValue] = &[
    FeedbackLabelValue::DetectionRoute(DetectionRouteLabel::ProviderRejected),
    FeedbackLabelValue::SystemOutcome(SystemOutcomeLabel::FailedValidation),
];

pub(super) fn quiet_mapping(reason: &str) -> Result<QuietMapping, ScanSelectedChatsError> {
    match reason {
        "deterministic_stop:unsupported_broad_content" => Ok(deterministic_stop(
            FeedbackEventType::DeterministicStopUnsupportedBroadContent,
        )),
        "deterministic_stop:invalid_evidence" => Ok(deterministic_stop(
            FeedbackEventType::DeterministicStopInvalidEvidence,
        )),
        "deterministic_stop:no_scheduling_signal" => Ok(deterministic_stop(
            FeedbackEventType::DeterministicStopNoSchedulingSignal,
        )),
        "deterministic_stop:past_or_invalid_time" => Ok(deterministic_stop(
            FeedbackEventType::DeterministicStopPastOrInvalidTime,
        )),
        "confidence_below_threshold" => Ok(QuietMapping {
            event_type: FeedbackEventType::ConfidenceBelowThreshold,
            snapshot_route: "provider_rejection",
            labels: PROVIDER_REJECTED_UNKNOWN_LABELS,
        }),
        "provider_unavailable" => Ok(QuietMapping {
            event_type: FeedbackEventType::ProviderUnavailable,
            snapshot_route: "provider_rejection",
            labels: PROVIDER_UNAVAILABLE_LABELS,
        }),
        "provider_invalid_json" => Ok(validation_failure(FeedbackEventType::ProviderInvalidJson)),
        "provider_schema_rejected" => Ok(validation_failure(
            FeedbackEventType::ProviderSchemaRejected,
        )),
        "provider_hallucinated_evidence" => Ok(validation_failure(
            FeedbackEventType::ProviderHallucinatedEvidence,
        )),
        "parser_provider_time_conflict" => Ok(validation_failure(
            FeedbackEventType::ParserProviderTimeConflict,
        )),
        other => Err(ScanSelectedChatsError::Detection(format!(
            "unsupported quiet feedback reason {other}"
        ))),
    }
}

const fn deterministic_stop(event_type: FeedbackEventType) -> QuietMapping {
    QuietMapping {
        event_type,
        snapshot_route: "deterministic_stop",
        labels: QUIET_STOP_LABELS,
    }
}

const fn validation_failure(event_type: FeedbackEventType) -> QuietMapping {
    QuietMapping {
        event_type,
        snapshot_route: "provider_rejection",
        labels: PROVIDER_VALIDATION_LABELS,
    }
}

pub(super) fn quiet_subject_id(quiet_log: &QuietLogDraft) -> String {
    format!(
        "quiet:{}:{}:{}",
        quiet_log.chat_guid, quiet_log.anchor_message_guid, quiet_log.reason
    )
}
