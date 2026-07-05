use morrow_storage::{CandidateKind, ExternalObjectMapping};

const EXTERNAL_PROPOSAL_CREATION_FAILED: &str = "external_proposal_creation_failed";
const MAX_CANDIDATE_REASON_BYTES: usize = 240;

#[derive(Debug, Clone)]
pub(super) enum CandidateReplayDecision {
    MappingPresent(ExternalObjectMapping),
    NeedsCalendarCreate,
    NeedsReminderCreate,
    NeedsLegacyCreate,
}

#[derive(Debug, Clone)]
pub(super) struct ReplayMapping {
    pub(super) mapping: ExternalObjectMapping,
    pub(super) created_external: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ReplayStoreDecision {
    TransitionVisible {
        summary_delta: ProposalReplayDelta,
    },
    MarkRecoveryPending {
        summary_delta: ProposalReplayDelta,
    },
    MarkFailed {
        reason: String,
        summary_delta: ProposalReplayDelta,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ProposalReplayDelta {
    pub(super) created: usize,
    pub(super) failed: usize,
}

pub(super) fn candidate_replay_decision(
    kind: CandidateKind,
    mapping: Option<ExternalObjectMapping>,
) -> CandidateReplayDecision {
    match mapping {
        Some(mapping) => CandidateReplayDecision::MappingPresent(mapping),
        None => match kind {
            CandidateKind::CalendarEvent => CandidateReplayDecision::NeedsCalendarCreate,
            CandidateKind::TaskReminder => CandidateReplayDecision::NeedsReminderCreate,
            CandidateKind::EventUpdate
            | CandidateKind::EventReschedule
            | CandidateKind::EventCancellation
            | CandidateKind::ReminderUpdate
            | CandidateKind::ReminderReschedule
            | CandidateKind::ReminderCancellation => CandidateReplayDecision::NeedsLegacyCreate,
        },
    }
}

pub(super) fn mapping_finalization_decision(
    created_external: bool,
    finalized: bool,
) -> ReplayStoreDecision {
    match (created_external, finalized) {
        (true, true) => ReplayStoreDecision::TransitionVisible {
            summary_delta: ProposalReplayDelta {
                created: 1,
                failed: 0,
            },
        },
        (false, true) => ReplayStoreDecision::TransitionVisible {
            summary_delta: ProposalReplayDelta {
                created: 0,
                failed: 0,
            },
        },
        (true, false) | (false, false) => ReplayStoreDecision::MarkRecoveryPending {
            summary_delta: ProposalReplayDelta {
                created: 0,
                failed: 1,
            },
        },
    }
}

pub(super) fn creation_failure_decision(detail: Option<&str>) -> ReplayStoreDecision {
    ReplayStoreDecision::MarkFailed {
        reason: external_proposal_failure_reason(detail),
        summary_delta: ProposalReplayDelta {
            created: 0,
            failed: 1,
        },
    }
}

fn external_proposal_failure_reason(message: Option<&str>) -> String {
    let Some(message) = message else {
        return EXTERNAL_PROPOSAL_CREATION_FAILED.to_owned();
    };
    let detail = truncated_failure_detail(message);
    if detail.is_empty() {
        return EXTERNAL_PROPOSAL_CREATION_FAILED.to_owned();
    }
    format!("{EXTERNAL_PROPOSAL_CREATION_FAILED}: {detail}")
}

fn truncated_failure_detail(message: &str) -> String {
    let mut detail = String::new();
    let max_detail_bytes = MAX_CANDIDATE_REASON_BYTES
        .saturating_sub(EXTERNAL_PROPOSAL_CREATION_FAILED.len())
        .saturating_sub(2);
    for ch in message.chars().filter(|ch| !ch.is_control()) {
        let next_len = detail.len() + ch.len_utf8();
        if next_len > max_detail_bytes {
            break;
        }
        detail.push(ch);
    }
    detail
}
