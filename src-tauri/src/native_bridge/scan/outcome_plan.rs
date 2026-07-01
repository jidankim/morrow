use morrow_detection::{DetectionOutcome, SourceExcerptPolicy};
use morrow_messages::MessageEvidence;
use morrow_storage::{CandidateDraft, QuietLogDraft};

use super::super::scan_privacy::{privacy_safe_candidate, privacy_safe_quiet_log};
use super::ScanSelectedChatsError;

pub(super) struct ScanOutcomePlan<'a> {
    pub(super) intents: Vec<ScanPersistenceIntent<'a>>,
}

pub(super) struct ScanOutcomePlanRequest<'a> {
    pub(super) outcomes: Vec<DetectionOutcome>,
    pub(super) messages: &'a [MessageEvidence],
    pub(super) trace_group_count: usize,
    pub(super) source_excerpts: SourceExcerptPolicy,
}

#[derive(Debug)]
pub(super) enum ScanPersistenceIntent<'a> {
    Candidate(PlannedCandidatePersistence<'a>),
    QuietLog(PlannedQuietLogPersistence<'a>),
}

#[derive(Debug)]
pub(super) struct PlannedCandidatePersistence<'a> {
    pub(super) candidate: CandidateDraft,
    pub(super) message: &'a MessageEvidence,
    pub(super) trace_group_index: Option<usize>,
}

#[derive(Debug)]
pub(super) struct PlannedQuietLogPersistence<'a> {
    pub(super) quiet_log: QuietLogDraft,
    pub(super) message: &'a MessageEvidence,
    pub(super) trace_group_index: Option<usize>,
}

pub(super) fn plan_scan_outcomes<'a>(
    request: ScanOutcomePlanRequest<'a>,
) -> Result<ScanOutcomePlan<'a>, ScanSelectedChatsError> {
    let ScanOutcomePlanRequest {
        outcomes,
        messages,
        trace_group_count,
        source_excerpts,
    } = request;
    let mut intents = Vec::with_capacity(outcomes.len());
    for (index, outcome) in outcomes.into_iter().enumerate() {
        let Some(message) = messages.get(index) else {
            return Err(ScanSelectedChatsError::Detection(
                "detection outcome missing message evidence".to_owned(),
            ));
        };
        let trace_group_index = (index < trace_group_count).then_some(index);
        match outcome {
            DetectionOutcome::Candidate(candidate) => {
                intents.push(ScanPersistenceIntent::Candidate(
                    PlannedCandidatePersistence {
                        candidate: privacy_safe_candidate(candidate, source_excerpts)?,
                        message,
                        trace_group_index,
                    },
                ));
            }
            DetectionOutcome::QuietLog(quiet_log) => {
                intents.push(ScanPersistenceIntent::QuietLog(
                    PlannedQuietLogPersistence {
                        quiet_log: privacy_safe_quiet_log(quiet_log, source_excerpts)?,
                        message,
                        trace_group_index,
                    },
                ));
            }
            DetectionOutcome::CachedProviderRoute { .. } => {}
        }
    }
    Ok(ScanOutcomePlan { intents })
}
