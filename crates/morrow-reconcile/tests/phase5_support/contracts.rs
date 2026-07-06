use morrow_storage::FeedbackPrivacyTier;

use super::schema::ExpectedOutcome;

pub(crate) type LabelContract = (&'static str, &'static str);
pub(crate) type TraceContract = (
    &'static str,
    &'static str,
    &'static str,
    FeedbackPrivacyTier,
);

pub(crate) const OUTCOME_ACCEPTED: [ExpectedOutcome; 1] = [ExpectedOutcome::ProposalAccepted];
pub(crate) const OUTCOME_REJECTED: [ExpectedOutcome; 1] = [ExpectedOutcome::ProposalRejected];
pub(crate) const OUTCOME_EDITED: [ExpectedOutcome; 1] =
    [ExpectedOutcome::ProposalEditedBeforeApproval];
pub(crate) const OUTCOME_QUIET: [ExpectedOutcome; 1] = [ExpectedOutcome::QuietLowConfidence];
pub(crate) const OUTCOME_COLLATERAL: [ExpectedOutcome; 2] = [
    ExpectedOutcome::ProposalAccepted,
    ExpectedOutcome::NonTargetPreserved,
];
pub(crate) const OUTCOME_REPLAY: [ExpectedOutcome; 2] = [
    ExpectedOutcome::ProposalAccepted,
    ExpectedOutcome::ReplayIdempotent,
];
pub(crate) const OUTCOME_CANARY: [ExpectedOutcome; 1] = [ExpectedOutcome::PrivacyCanaryRejected];

pub(crate) const LABEL_ACCEPTED: [LabelContract; 3] = [
    ("proposal_outcome", "accepted"),
    ("detection_route", "deterministic_candidate"),
    ("system_outcome", "ok"),
];
pub(crate) const LABEL_REJECTED: [LabelContract; 3] = [
    ("proposal_outcome", "rejected_observed"),
    ("detection_route", "deterministic_candidate"),
    ("system_outcome", "ok"),
];
pub(crate) const LABEL_EDITED: [LabelContract; 4] = [
    ("proposal_outcome", "pending_edited"),
    ("field_quality", "title_edited"),
    ("field_quality", "time_edited"),
    ("system_outcome", "ok"),
];
pub(crate) const LABEL_QUIET: [LabelContract; 3] = [
    ("proposal_outcome", "unknown"),
    ("detection_route", "quiet_stop"),
    ("system_outcome", "ok"),
];
pub(crate) const LABEL_CANARY: [LabelContract; 3] = [
    ("detection_route", "provider_rejected"),
    ("system_outcome", "failed_validation"),
    ("proposal_outcome", "unknown"),
];

pub(crate) const TRACE_ACCEPTED: [TraceContract; 4] = [
    (
        "parser",
        "parser_decision",
        "candidate_created",
        FeedbackPrivacyTier::InternalMetadata,
    ),
    (
        "outcome",
        "outcome_materialized",
        "candidate_created",
        FeedbackPrivacyTier::InternalMetadata,
    ),
    (
        "lifecycle",
        "approval_observed",
        "approved",
        FeedbackPrivacyTier::InternalMetadata,
    ),
    (
        "cleanup",
        "cleanup_verification",
        "cleaned",
        FeedbackPrivacyTier::InternalMetadata,
    ),
];
pub(crate) const TRACE_REJECTED: [TraceContract; 4] = [
    TRACE_ACCEPTED[0],
    TRACE_ACCEPTED[1],
    (
        "lifecycle",
        "rejection_observed",
        "rejected",
        FeedbackPrivacyTier::InternalMetadata,
    ),
    TRACE_ACCEPTED[3],
];
pub(crate) const TRACE_EDITED: [TraceContract; 4] = [
    TRACE_ACCEPTED[0],
    TRACE_ACCEPTED[1],
    (
        "lifecycle",
        "pending_edit_observed",
        "edited",
        FeedbackPrivacyTier::InternalMetadata,
    ),
    (
        "cleanup",
        "cleanup_verification",
        "preserved",
        FeedbackPrivacyTier::InternalMetadata,
    ),
];
pub(crate) const TRACE_QUIET: [TraceContract; 4] = [
    (
        "parser",
        "parser_decision",
        "provider_route",
        FeedbackPrivacyTier::InternalMetadata,
    ),
    (
        "provider",
        "provider_result",
        "low_confidence",
        FeedbackPrivacyTier::InternalMetadata,
    ),
    (
        "threshold",
        "threshold_decision",
        "quiet_logged",
        FeedbackPrivacyTier::InternalMetadata,
    ),
    (
        "cleanup",
        "cleanup_verification",
        "no_external_created",
        FeedbackPrivacyTier::InternalMetadata,
    ),
];
pub(crate) const TRACE_CANARY: [TraceContract; 3] = [
    (
        "schema",
        "schema_validation",
        "rejected",
        FeedbackPrivacyTier::InternalMetadata,
    ),
    (
        "privacy",
        "privacy_canary_rejection",
        "rejected",
        FeedbackPrivacyTier::InternalMetadata,
    ),
    TRACE_QUIET[3],
];
