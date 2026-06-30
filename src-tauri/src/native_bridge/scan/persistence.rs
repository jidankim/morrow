use morrow_storage::{CandidateId, Store};

use super::{
    outcome_plan::{
        PlannedCandidatePersistence, PlannedQuietLogPersistence, ScanPersistenceIntent,
    },
    storage_error, ScanSelectedChatsError,
};

pub(super) struct ScanPersistenceRequest<'a, C, Q> {
    pub(super) store: &'a Store,
    pub(super) intents: Vec<ScanPersistenceIntent<'a>>,
    pub(super) feedback: ScanPersistenceFeedback<C, Q>,
}

pub(super) struct ScanPersistenceFeedback<C, Q> {
    pub(super) record_candidate: C,
    pub(super) record_quiet_log: Q,
}

pub(super) struct CandidatePersistenceFeedback<'plan, 'item> {
    pub(super) candidate_id: &'item CandidateId,
    pub(super) planned: &'item PlannedCandidatePersistence<'plan>,
}

pub(super) struct QuietLogPersistenceFeedback<'plan, 'item> {
    pub(super) planned: &'item PlannedQuietLogPersistence<'plan>,
}

pub(super) struct ScanPersistenceResult {
    pub(super) created_candidate_ids: Vec<CandidateId>,
    pub(super) quiet_log_count: usize,
}

pub(super) fn apply_scan_persistence<'plan, C, Q>(
    request: ScanPersistenceRequest<'plan, C, Q>,
) -> Result<ScanPersistenceResult, ScanSelectedChatsError>
where
    C: for<'item> FnMut(
        CandidatePersistenceFeedback<'plan, 'item>,
    ) -> Result<(), ScanSelectedChatsError>,
    Q: for<'item> FnMut(
        QuietLogPersistenceFeedback<'plan, 'item>,
    ) -> Result<(), ScanSelectedChatsError>,
{
    let ScanPersistenceRequest {
        store,
        intents,
        mut feedback,
    } = request;
    let mut created_candidate_ids = Vec::new();
    let mut quiet_log_count = 0;
    for intent in intents {
        match intent {
            ScanPersistenceIntent::Candidate(planned) => {
                let candidate_id = store
                    .create_candidate(planned.candidate.clone())
                    .map_err(storage_error)?;
                (feedback.record_candidate)(CandidatePersistenceFeedback {
                    candidate_id: &candidate_id,
                    planned: &planned,
                })?;
                created_candidate_ids.push(candidate_id);
            }
            ScanPersistenceIntent::QuietLog(planned) => {
                store
                    .record_quiet_log(planned.quiet_log.clone())
                    .map_err(storage_error)?;
                (feedback.record_quiet_log)(QuietLogPersistenceFeedback { planned: &planned })?;
                quiet_log_count += 1;
            }
        }
    }
    Ok(ScanPersistenceResult {
        created_candidate_ids,
        quiet_log_count,
    })
}
