use morrow_detection::{AiProvider, ProviderError, ProviderRequest, ProviderResponse};
use morrow_diagnostics::TraceRecorder;
use morrow_messages::MessagesDataSource;

use super::{ProposalReplayAdapter, ScanSelectedChatsResult};

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

pub(super) const fn empty_scan_result() -> ScanSelectedChatsResult {
    ScanSelectedChatsResult {
        pending_proposal_count: 0,
        created_candidate_count: 0,
        quiet_log_count: 0,
        cap_visible_count: 0,
        cap_deferred_count: 0,
        created_external_proposal_count: 0,
        failed_external_proposal_count: 0,
        created_candidate_ids: Vec::new(),
    }
}
