mod adapter;
mod calendar;
mod decision;
mod finalize;
#[cfg(test)]
mod tests;

use morrow_storage::{CandidateKind, ExternalObjectMapping, ExternalSource, QueuedProposal, Store};

use super::{storage_error, ScanSelectedChatsError};
pub(in crate::native_bridge) use adapter::LocalProposalAdapter;
pub use adapter::ProposalReplayAdapter;
use calendar::calendar_mapping_from_store;
pub use calendar::CalendarProposalReceipt;
use decision::{
    candidate_replay_decision, creation_failure_decision, mapping_finalization_decision,
    CandidateReplayDecision, ProposalReplayDelta, ReplayMapping,
};
use finalize::{apply_store_decision, finalize_existing_mapping, transition_mapping_visible};

const MAPPED_AT: i64 = 1_782_352_400;
const DEFAULT_CALENDAR_EVENT_DURATION_SECONDS: i64 = 30 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct ProposalReplaySummary {
    pub(super) created: usize,
    pub(super) failed: usize,
    pub(super) dry_run: usize,
    pub(super) calendar_commit_idempotency: usize,
}

pub(super) fn replay_external_proposals(
    store: &Store,
    candidates: &[QueuedProposal],
    adapter: &impl ProposalReplayAdapter,
) -> Result<ProposalReplaySummary, ScanSelectedChatsError> {
    replay_external_proposals_with_mode(store, candidates, adapter, ReplayMode::Commit)
}

#[cfg(test)]
pub(super) fn replay_external_proposals_dry_run(
    store: &Store,
    candidates: &[QueuedProposal],
    adapter: &impl ProposalReplayAdapter,
) -> Result<ProposalReplaySummary, ScanSelectedChatsError> {
    replay_external_proposals_with_mode(store, candidates, adapter, ReplayMode::DryRun)
}

fn replay_external_proposals_with_mode(
    store: &Store,
    candidates: &[QueuedProposal],
    adapter: &impl ProposalReplayAdapter,
    mode: ReplayMode,
) -> Result<ProposalReplaySummary, ScanSelectedChatsError> {
    let mut summary = ProposalReplaySummary::default();
    for candidate in candidates {
        let replay_result =
            match candidate_replay_decision(
                candidate.kind,
                mapping_for_candidate(store, candidate)?,
            ) {
                CandidateReplayDecision::MappingPresent(mapping) => {
                    if mode == ReplayMode::DryRun {
                        summary.dry_run += 1;
                        continue;
                    }
                    finalize_existing_mapping(store, candidate.kind, &mapping, &mut summary)?;
                    continue;
                }
                CandidateReplayDecision::NeedsCalendarCreate => match mode {
                    ReplayMode::Commit => calendar_mapping_from_store(store, candidate, adapter)
                        .map(|mapping| ReplayMapping {
                            mapping,
                            created_external: true,
                        }),
                    ReplayMode::DryRun => {
                        summary.dry_run += 1;
                        continue;
                    }
                },
                CandidateReplayDecision::NeedsLegacyCreate => match mode {
                    ReplayMode::Commit => {
                        adapter
                            .create_legacy_proposal(candidate)
                            .map(|mapping| ReplayMapping {
                                mapping,
                                created_external: true,
                            })
                    }
                    ReplayMode::DryRun => {
                        summary.dry_run += 1;
                        continue;
                    }
                },
            };
        match replay_result {
            Ok(replay_mapping) => {
                if replay_mapping.created_external {
                    store
                        .record_candidate_external_receipt(&replay_mapping.mapping)
                        .map_err(storage_error)?;
                }
                let finalized = transition_mapping_visible(store, &replay_mapping.mapping);
                let store_decision =
                    mapping_finalization_decision(replay_mapping.created_external, finalized);
                summary.apply(apply_store_decision(
                    store,
                    &replay_mapping.mapping.candidate_id,
                    store_decision,
                )?);
            }
            Err(error) => {
                let store_decision =
                    creation_failure_decision(external_proposal_failure_detail(&error));
                summary.apply(apply_store_decision(
                    store,
                    &candidate.candidate_id,
                    store_decision,
                )?);
            }
        }
    }
    Ok(summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReplayMode {
    Commit,
    DryRun,
}

fn mapping_for_candidate(
    store: &Store,
    candidate: &QueuedProposal,
) -> Result<Option<ExternalObjectMapping>, ScanSelectedChatsError> {
    match candidate.kind {
        CandidateKind::CalendarEvent => store
            .candidate_external_mapping(
                &candidate.candidate_id,
                ExternalSource::Calendar,
                MAPPED_AT,
            )
            .map_err(storage_error),
        CandidateKind::TaskReminder
        | CandidateKind::EventUpdate
        | CandidateKind::EventReschedule
        | CandidateKind::EventCancellation
        | CandidateKind::ReminderUpdate
        | CandidateKind::ReminderReschedule
        | CandidateKind::ReminderCancellation => Ok(None),
    }
}

impl ProposalReplaySummary {
    const fn apply(&mut self, delta: ProposalReplayDelta) {
        self.created += delta.created;
        self.failed += delta.failed;
    }
}

fn external_proposal_error(message: impl Into<String>) -> ScanSelectedChatsError {
    ScanSelectedChatsError::ExternalProposal(message.into())
}

fn external_proposal_failure_detail(error: &ScanSelectedChatsError) -> Option<&str> {
    match error {
        ScanSelectedChatsError::ExternalProposal(message) => Some(message.as_str()),
        ScanSelectedChatsError::Detection(_)
        | ScanSelectedChatsError::Messages(_)
        | ScanSelectedChatsError::Storage(_) => None,
    }
}
