mod calendar;
#[cfg(test)]
mod tests;

use morrow_calendar::ProposedEvent;
use morrow_storage::{
    CandidateKind, CandidateState, ExternalObjectMapping, ExternalSource, QueuedProposal, Store,
};

use super::super::eventkit_proposal::EventKitProposalBridge;
use super::{storage_error, ScanSelectedChatsError};
use calendar::calendar_mapping_from_store;
pub use calendar::CalendarProposalReceipt;

const MAPPED_AT: i64 = 1_782_352_400;
const DEFAULT_CALENDAR_EVENT_DURATION_SECONDS: i64 = 30 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ProposalReplaySummary {
    pub(super) created: usize,
    pub(super) failed: usize,
}

pub(super) struct LocalProposalAdapter;

pub trait ProposalReplayAdapter {
    fn create_calendar_proposal(
        &self,
        event: ProposedEvent,
    ) -> Result<CalendarProposalReceipt, ScanSelectedChatsError>;

    fn create_legacy_proposal(
        &self,
        candidate: &QueuedProposal,
    ) -> Result<ExternalObjectMapping, ScanSelectedChatsError>;
}

impl ProposalReplayAdapter for LocalProposalAdapter {
    fn create_calendar_proposal(
        &self,
        event: ProposedEvent,
    ) -> Result<CalendarProposalReceipt, ScanSelectedChatsError> {
        Ok(CalendarProposalReceipt {
            event_id: format!(
                "morrow-local-calendar-{}",
                event.metadata.candidate_id.as_str()
            ),
            source_id: event.metadata.source_id.as_str().to_owned(),
        })
    }

    fn create_legacy_proposal(
        &self,
        candidate: &QueuedProposal,
    ) -> Result<ExternalObjectMapping, ScanSelectedChatsError> {
        let source = match candidate.kind {
            CandidateKind::CalendarEvent => {
                return Err(external_proposal_error(
                    "calendar candidates require calendar proposal payload replay",
                ));
            }
            CandidateKind::TaskReminder => ExternalSource::Reminders,
            CandidateKind::EventUpdate
            | CandidateKind::EventReschedule
            | CandidateKind::EventCancellation
            | CandidateKind::ReminderUpdate
            | CandidateKind::ReminderReschedule
            | CandidateKind::ReminderCancellation => {
                return Err(external_proposal_error(
                    "proposal creation is unavailable for mutation candidates",
                ));
            }
        };
        Ok(ExternalObjectMapping {
            candidate_id: candidate.candidate_id.clone(),
            source,
            external_object_id: format!(
                "morrow-local-{}-{}",
                source.as_str(),
                candidate.candidate_id.as_str()
            ),
            external_source_id: format!("morrow-local-{}", source.as_str()),
            mapped_at: MAPPED_AT,
        })
    }
}

impl ProposalReplayAdapter for EventKitProposalBridge {
    fn create_calendar_proposal(
        &self,
        event: ProposedEvent,
    ) -> Result<CalendarProposalReceipt, ScanSelectedChatsError> {
        self.propose_event(event)
            .map(|receipt| CalendarProposalReceipt {
                event_id: receipt.event_id,
                source_id: receipt.source_id,
            })
            .map_err(|error| external_proposal_error(error.to_string()))
    }

    fn create_legacy_proposal(
        &self,
        _candidate: &QueuedProposal,
    ) -> Result<ExternalObjectMapping, ScanSelectedChatsError> {
        Err(external_proposal_error(
            "EventKit proposal adapter cannot create legacy proposal kinds",
        ))
    }
}

pub(super) fn replay_external_proposals(
    store: &Store,
    candidates: &[QueuedProposal],
    adapter: &impl ProposalReplayAdapter,
) -> Result<ProposalReplaySummary, ScanSelectedChatsError> {
    let mut created = 0;
    let mut failed = 0;
    for candidate in candidates {
        let replay_result = match mapping_for_candidate(store, candidate)? {
            Some(mapping) => Ok(ReplayMapping {
                mapping,
                created_external: false,
            }),
            None => match candidate.kind {
                CandidateKind::CalendarEvent => {
                    calendar_mapping_from_store(store, candidate, adapter).map(|mapping| {
                        ReplayMapping {
                            mapping,
                            created_external: true,
                        }
                    })
                }
                CandidateKind::TaskReminder
                | CandidateKind::EventUpdate
                | CandidateKind::EventReschedule
                | CandidateKind::EventCancellation
                | CandidateKind::ReminderUpdate
                | CandidateKind::ReminderReschedule
                | CandidateKind::ReminderCancellation => adapter
                    .create_legacy_proposal(candidate)
                    .map(|mapping| ReplayMapping {
                        mapping,
                        created_external: true,
                    }),
            },
        };
        match replay_result {
            Ok(replay_mapping) => {
                if replay_mapping.created_external {
                    store
                        .record_candidate_external_receipt(&replay_mapping.mapping)
                        .map_err(storage_error)?;
                }
                if finalize_external_mapping(store, &replay_mapping.mapping)? {
                    if replay_mapping.created_external {
                        created += 1;
                    }
                } else {
                    failed += 1;
                }
            }
            Err(_error) => {
                store
                    .transition_candidate(
                        &candidate.candidate_id,
                        CandidateState::Failed,
                        "external_proposal_creation_failed",
                        MAPPED_AT,
                    )
                    .map_err(storage_error)?;
                failed += 1;
            }
        }
    }
    Ok(ProposalReplaySummary { created, failed })
}

#[derive(Debug, Clone)]
struct ReplayMapping {
    mapping: ExternalObjectMapping,
    created_external: bool,
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

fn finalize_external_mapping(
    store: &Store,
    mapping: &ExternalObjectMapping,
) -> Result<bool, ScanSelectedChatsError> {
    let finalized = store
        .upsert_external_mapping(mapping.clone())
        .and_then(|()| {
            store.transition_candidate(
                &mapping.candidate_id,
                CandidateState::Visible,
                "external_proposal_created",
                MAPPED_AT,
            )
        });
    match finalized {
        Ok(()) => Ok(true),
        Err(_error) => {
            store
                .record_external_replay_recovery_pending(
                    &mapping.candidate_id,
                    "external_proposal_recovery_pending",
                    MAPPED_AT,
                )
                .map_err(storage_error)?;
            Ok(false)
        }
    }
}

fn external_proposal_error(message: impl Into<String>) -> ScanSelectedChatsError {
    ScanSelectedChatsError::ExternalProposal(message.into())
}
