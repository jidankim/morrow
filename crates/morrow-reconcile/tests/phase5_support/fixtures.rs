// allow: SIZE_OK — static Phase 5 fixture taxonomy table; splitting cases would obscure coverage.
use morrow_storage::{
    CandidateKind, DetectionRouteLabel, FeedbackLabelType, FeedbackPrivacyTier, FieldQualityLabel,
    ProposalOutcomeLabel, SystemOutcomeLabel,
};

use super::schema::{
    AllowedSideEffect, CleanupAction, CleanupExpectation, ExpectedLabel, ExpectedOutcome,
    ExpectedTraceOperation, ProposalKindExpectation, ScoreWeight, TrajectoryCase,
};

const WEIGHTS: [ScoreWeight; 5] = [
    ScoreWeight {
        category: "route_correctness",
        weight: 25,
    },
    ScoreWeight {
        category: "proposal_outcome",
        weight: 25,
    },
    ScoreWeight {
        category: "trace_coverage",
        weight: 20,
    },
    ScoreWeight {
        category: "collateral_damage",
        weight: 15,
    },
    ScoreWeight {
        category: "cleanup_privacy",
        weight: 15,
    },
];

const TRACE_ACCEPTED: [ExpectedTraceOperation; 4] = [
    trace("parser", "parser_decision", "candidate_created"),
    trace("outcome", "outcome_materialized", "candidate_created"),
    trace("lifecycle", "approval_observed", "approved"),
    trace("cleanup", "cleanup_verification", "cleaned"),
];
const TRACE_REJECTED: [ExpectedTraceOperation; 4] = [
    trace("parser", "parser_decision", "candidate_created"),
    trace("outcome", "outcome_materialized", "candidate_created"),
    trace("lifecycle", "rejection_observed", "rejected"),
    trace("cleanup", "cleanup_verification", "cleaned"),
];
const TRACE_EDITED: [ExpectedTraceOperation; 4] = [
    trace("parser", "parser_decision", "candidate_created"),
    trace("outcome", "outcome_materialized", "candidate_created"),
    trace("lifecycle", "pending_edit_observed", "edited"),
    trace("cleanup", "cleanup_verification", "preserved"),
];
const TRACE_QUIET: [ExpectedTraceOperation; 4] = [
    trace("parser", "parser_decision", "provider_route"),
    trace("provider", "provider_result", "low_confidence"),
    trace("threshold", "threshold_decision", "quiet_logged"),
    trace("cleanup", "cleanup_verification", "no_external_created"),
];
const TRACE_CANARY: [ExpectedTraceOperation; 3] = [
    trace("schema", "schema_validation", "rejected"),
    trace("privacy", "privacy_canary_rejection", "rejected"),
    trace("cleanup", "cleanup_verification", "no_external_created"),
];

const LABEL_ACCEPTED: [ExpectedLabel; 3] = [
    label(
        FeedbackLabelType::ProposalOutcome,
        ProposalOutcomeLabel::Accepted.as_str(),
    ),
    label(
        FeedbackLabelType::DetectionRoute,
        DetectionRouteLabel::DeterministicCandidate.as_str(),
    ),
    label(
        FeedbackLabelType::SystemOutcome,
        SystemOutcomeLabel::Ok.as_str(),
    ),
];
const LABEL_REJECTED: [ExpectedLabel; 3] = [
    label(
        FeedbackLabelType::ProposalOutcome,
        ProposalOutcomeLabel::RejectedObserved.as_str(),
    ),
    label(
        FeedbackLabelType::DetectionRoute,
        DetectionRouteLabel::DeterministicCandidate.as_str(),
    ),
    label(
        FeedbackLabelType::SystemOutcome,
        SystemOutcomeLabel::Ok.as_str(),
    ),
];
const LABEL_EDITED: [ExpectedLabel; 4] = [
    label(
        FeedbackLabelType::ProposalOutcome,
        ProposalOutcomeLabel::PendingEdited.as_str(),
    ),
    label(
        FeedbackLabelType::FieldQuality,
        FieldQualityLabel::TitleEdited.as_str(),
    ),
    label(
        FeedbackLabelType::FieldQuality,
        FieldQualityLabel::TimeEdited.as_str(),
    ),
    label(
        FeedbackLabelType::SystemOutcome,
        SystemOutcomeLabel::Ok.as_str(),
    ),
];
const LABEL_QUIET: [ExpectedLabel; 3] = [
    label(
        FeedbackLabelType::ProposalOutcome,
        ProposalOutcomeLabel::Unknown.as_str(),
    ),
    label(
        FeedbackLabelType::DetectionRoute,
        DetectionRouteLabel::QuietStop.as_str(),
    ),
    label(
        FeedbackLabelType::SystemOutcome,
        SystemOutcomeLabel::Ok.as_str(),
    ),
];
const LABEL_CANARY: [ExpectedLabel; 3] = [
    label(
        FeedbackLabelType::DetectionRoute,
        DetectionRouteLabel::ProviderRejected.as_str(),
    ),
    label(
        FeedbackLabelType::SystemOutcome,
        SystemOutcomeLabel::FailedValidation.as_str(),
    ),
    label(
        FeedbackLabelType::ProposalOutcome,
        ProposalOutcomeLabel::Unknown.as_str(),
    ),
];

const OUTCOME_ACCEPTED: [ExpectedOutcome; 1] = [ExpectedOutcome::ProposalAccepted];
const OUTCOME_REJECTED: [ExpectedOutcome; 1] = [ExpectedOutcome::ProposalRejected];
const OUTCOME_EDITED: [ExpectedOutcome; 1] = [ExpectedOutcome::ProposalEditedBeforeApproval];
const OUTCOME_QUIET: [ExpectedOutcome; 1] = [ExpectedOutcome::QuietLowConfidence];
const OUTCOME_COLLATERAL: [ExpectedOutcome; 2] = [
    ExpectedOutcome::ProposalAccepted,
    ExpectedOutcome::NonTargetPreserved,
];
const OUTCOME_REPLAY: [ExpectedOutcome; 2] = [
    ExpectedOutcome::ProposalAccepted,
    ExpectedOutcome::ReplayIdempotent,
];
const OUTCOME_CANARY: [ExpectedOutcome; 1] = [ExpectedOutcome::PrivacyCanaryRejected];

const SIDE_CAL_ACCEPT: [AllowedSideEffect; 3] = [
    AllowedSideEffect::CreateCalendarProposal,
    AllowedSideEffect::RemoveProposedExternal,
    AllowedSideEffect::RecordFeedbackOnly,
];
const SIDE_REMINDER_ACCEPT: [AllowedSideEffect; 3] = [
    AllowedSideEffect::CreateReminderProposal,
    AllowedSideEffect::RemoveProposedExternal,
    AllowedSideEffect::RecordFeedbackOnly,
];
const SIDE_REJECT: [AllowedSideEffect; 2] = [
    AllowedSideEffect::RemoveProposedExternal,
    AllowedSideEffect::RecordFeedbackOnly,
];
const SIDE_QUIET: [AllowedSideEffect; 2] = [
    AllowedSideEffect::RecordFeedbackOnly,
    AllowedSideEffect::RejectBeforeMutation,
];
const SIDE_COLLATERAL: [AllowedSideEffect; 4] = [
    AllowedSideEffect::CreateCalendarProposal,
    AllowedSideEffect::RemoveProposedExternal,
    AllowedSideEffect::PreserveNonTargetExternal,
    AllowedSideEffect::RecordFeedbackOnly,
];

pub(crate) const TRAJECTORY_CASES: [TrajectoryCase; 9] = [
    case(
        "phase5:scheduled_meeting_accepted:v1",
        "scheduled_meeting_accepted",
        CandidateKind::CalendarEvent,
        ProposalKindExpectation::CalendarEvent,
        &OUTCOME_ACCEPTED,
        &LABEL_ACCEPTED,
        &TRACE_ACCEPTED,
        &SIDE_CAL_ACCEPT,
        cleanup_delete(),
    ),
    case(
        "phase5:scheduled_meeting_rejected:v1",
        "scheduled_meeting_rejected",
        CandidateKind::CalendarEvent,
        ProposalKindExpectation::CalendarEvent,
        &OUTCOME_REJECTED,
        &LABEL_REJECTED,
        &TRACE_REJECTED,
        &SIDE_REJECT,
        cleanup_delete(),
    ),
    case(
        "phase5:scheduled_meeting_edited_before_approval:v1",
        "scheduled_meeting_edited_before_approval",
        CandidateKind::CalendarEvent,
        ProposalKindExpectation::CalendarEvent,
        &OUTCOME_EDITED,
        &LABEL_EDITED,
        &TRACE_EDITED,
        &SIDE_CAL_ACCEPT,
        cleanup_delete(),
    ),
    case(
        "phase5:task_reminder_accepted:v1",
        "task_reminder_accepted",
        CandidateKind::TaskReminder,
        ProposalKindExpectation::TaskReminder,
        &OUTCOME_ACCEPTED,
        &LABEL_ACCEPTED,
        &TRACE_ACCEPTED,
        &SIDE_REMINDER_ACCEPT,
        cleanup_delete(),
    ),
    case(
        "phase5:task_reminder_rejected:v1",
        "task_reminder_rejected",
        CandidateKind::TaskReminder,
        ProposalKindExpectation::TaskReminder,
        &OUTCOME_REJECTED,
        &LABEL_REJECTED,
        &TRACE_REJECTED,
        &SIDE_REJECT,
        cleanup_delete(),
    ),
    case(
        "phase5:provider_quiet_low_confidence:v1",
        "provider_quiet_low_confidence",
        CandidateKind::CalendarEvent,
        ProposalKindExpectation::NoProposal,
        &OUTCOME_QUIET,
        &LABEL_QUIET,
        &TRACE_QUIET,
        &SIDE_QUIET,
        cleanup_none(),
    ),
    case(
        "phase5:collateral_damage_non_target_preserved:v1",
        "collateral_damage_non_target_preserved",
        CandidateKind::CalendarEvent,
        ProposalKindExpectation::CalendarEvent,
        &OUTCOME_COLLATERAL,
        &LABEL_ACCEPTED,
        &TRACE_ACCEPTED,
        &SIDE_COLLATERAL,
        cleanup_preserve(),
    ),
    case(
        "phase5:replay_idempotent_retry:v1",
        "replay_idempotent_retry",
        CandidateKind::CalendarEvent,
        ProposalKindExpectation::CalendarEvent,
        &OUTCOME_REPLAY,
        &LABEL_ACCEPTED,
        &TRACE_ACCEPTED,
        &SIDE_CAL_ACCEPT,
        cleanup_delete(),
    ),
    case(
        "phase5:privacy_canary_rejection:v1",
        "privacy_canary_rejection",
        CandidateKind::CalendarEvent,
        ProposalKindExpectation::NoProposal,
        &OUTCOME_CANARY,
        &LABEL_CANARY,
        &TRACE_CANARY,
        &SIDE_QUIET,
        cleanup_none(),
    ),
];

const fn trace(
    component: &'static str,
    operation: &'static str,
    outcome: &'static str,
) -> ExpectedTraceOperation {
    ExpectedTraceOperation {
        component,
        operation,
        outcome,
        privacy_tier: FeedbackPrivacyTier::InternalMetadata,
    }
}

const fn label(label_type: FeedbackLabelType, label_value: &'static str) -> ExpectedLabel {
    ExpectedLabel {
        label_type: label_type.as_str(),
        label_value,
    }
}

const fn cleanup_delete() -> CleanupExpectation {
    CleanupExpectation {
        action: CleanupAction::DeleteProposedExternal,
        non_target_fixture_id: None,
        live_surface_used: false,
    }
}

const fn cleanup_none() -> CleanupExpectation {
    CleanupExpectation {
        action: CleanupAction::NoExternalCreated,
        non_target_fixture_id: None,
        live_surface_used: false,
    }
}

const fn cleanup_preserve() -> CleanupExpectation {
    CleanupExpectation {
        action: CleanupAction::PreserveNonTargetOnly,
        non_target_fixture_id: Some("phase5:non_target_external:preserved:v1"),
        live_surface_used: false,
    }
}

const fn case(
    trajectory_case_id: &'static str,
    family: &'static str,
    candidate_kind: CandidateKind,
    proposal_kind: ProposalKindExpectation,
    outcomes: &'static [ExpectedOutcome],
    labels: &'static [ExpectedLabel],
    trace_operations: &'static [ExpectedTraceOperation],
    side_effects: &'static [AllowedSideEffect],
    cleanup: CleanupExpectation,
) -> TrajectoryCase {
    TrajectoryCase {
        trajectory_case_id,
        family,
        source_fixture_id: "phase5:source_fixture:v1",
        candidate_fixture_id: "phase5:candidate_fixture:v1",
        proposal_fixture_id: "phase5:proposal_fixture:v1",
        bounded_excerpt_placeholder: "[bounded scheduling evidence placeholder]",
        expected_candidate_kind: candidate_kind,
        expected_proposal_kind: proposal_kind,
        expected_outcomes: outcomes,
        expected_labels: labels,
        expected_trace_operations: trace_operations,
        allowed_side_effects: side_effects,
        cleanup: Some(cleanup),
        score_weights: &WEIGHTS,
        explicit_score_total: None,
    }
}
