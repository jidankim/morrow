use morrow_detection::{AiProvider, ProviderError, ProviderRequest, ProviderResponse};
use morrow_diagnostics::TraceRecorder;
use morrow_messages::MessagesDataSource;

use morrow_storage::FeedbackEvalCounts;

use super::{LatestEvalStatus, ProposalReplayAdapter, ScanSelectedChatsResult};

pub struct ScanSelectedChatsDependencies<'a, S, P, A, R>
where
    S: MessagesDataSource,
    P: AiProvider,
    A: ProposalReplayAdapter,
    R: TraceRecorder + ?Sized,
{
    pub source: &'a S,
    pub provider: &'a P,
    pub proposal_adapter: &'a A,
    pub trace_recorder: &'a R,
}

pub(super) struct UnavailableProvider;

impl AiProvider for UnavailableProvider {
    fn extract(&self, _request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        Err(ProviderError::Unavailable {
            reason: "native scan provider is not configured".to_owned(),
        })
    }
}

pub(super) fn empty_scan_result(
    feedback_eval_counts: &FeedbackEvalCounts,
) -> Result<ScanSelectedChatsResult, super::ScanSelectedChatsError> {
    Ok(ScanSelectedChatsResult {
        pending_proposal_count: 0,
        created_candidate_count: 0,
        quiet_log_count: 0,
        cap_visible_count: 0,
        cap_deferred_count: 0,
        created_external_proposal_count: 0,
        failed_external_proposal_count: 0,
        feedback_label_count: super::count_to_usize(
            feedback_eval_counts.label_count,
            "label_count",
        )?,
        feature_snapshot_count: super::count_to_usize(
            feedback_eval_counts.feature_snapshot_count,
            "feature_snapshot_count",
        )?,
        latest_eval_status: LatestEvalStatus::from(feedback_eval_counts.latest_eval_status),
        created_candidate_ids: Vec::new(),
    })
}
