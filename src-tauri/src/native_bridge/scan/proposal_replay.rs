use morrow_storage::{
    CandidateKind, CandidateState, ExternalObjectMapping, ExternalSource, QueuedProposal, Store,
};

use super::{storage_error, ScanSelectedChatsError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ProposalReplaySummary {
    pub(super) created: usize,
    pub(super) failed: usize,
}

pub(super) struct LocalProposalAdapter;

trait ExternalProposalAdapter {
    fn create_proposal(
        &self,
        candidate: &QueuedProposal,
    ) -> Result<ExternalObjectMapping, ScanSelectedChatsError>;
}

impl ExternalProposalAdapter for LocalProposalAdapter {
    fn create_proposal(
        &self,
        candidate: &QueuedProposal,
    ) -> Result<ExternalObjectMapping, ScanSelectedChatsError> {
        let source = match candidate.kind {
            CandidateKind::CalendarEvent => ExternalSource::Calendar,
            CandidateKind::TaskReminder => ExternalSource::Reminders,
            CandidateKind::EventUpdate
            | CandidateKind::EventReschedule
            | CandidateKind::EventCancellation
            | CandidateKind::ReminderUpdate
            | CandidateKind::ReminderReschedule
            | CandidateKind::ReminderCancellation => {
                return Err(ScanSelectedChatsError::ExternalProposal(
                    "proposal creation is unavailable for mutation candidates".to_owned(),
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
            mapped_at: 1_782_352_400,
        })
    }
}

pub(super) fn replay_external_proposals(
    store: &Store,
    candidates: &[QueuedProposal],
    adapter: &LocalProposalAdapter,
) -> Result<ProposalReplaySummary, ScanSelectedChatsError> {
    let mut created = 0;
    let mut failed = 0;
    for candidate in candidates {
        match adapter.create_proposal(candidate) {
            Ok(mapping) => {
                store
                    .upsert_external_mapping(mapping)
                    .map_err(storage_error)?;
                store
                    .transition_candidate(
                        &candidate.candidate_id,
                        CandidateState::Visible,
                        "external_proposal_created",
                        1_782_352_400,
                    )
                    .map_err(storage_error)?;
                created += 1;
            }
            Err(_error) => {
                store
                    .transition_candidate(
                        &candidate.candidate_id,
                        CandidateState::Failed,
                        "external_proposal_creation_failed",
                        1_782_352_400,
                    )
                    .map_err(storage_error)?;
                failed += 1;
            }
        }
    }
    Ok(ProposalReplaySummary { created, failed })
}
