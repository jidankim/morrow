use std::path::Path;

use super::dependencies::UnavailableProvider;
use super::{
    messages_error, scan_selected_chats_with_dependencies, ProposalReplayAdapter,
    ScanSelectedChatsDependencies, ScanSelectedChatsError, ScanSelectedChatsRequest,
    ScanSelectedChatsResult,
};
use crate::native_bridge::messages_sqlite::MessagesSqliteAdapter;
use morrow_detection::AiProvider;
use morrow_diagnostics::NoopTraceRecorder;
use morrow_messages::ChatGuid;

pub fn scan_selected_chats_at_with_unavailable_provider<A>(
    request: ScanSelectedChatsRequest,
    store_path: &Path,
    messages_db_path: &Path,
    proposal_adapter: &A,
) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError>
where
    A: ProposalReplayAdapter,
{
    let provider = UnavailableProvider;
    scan_selected_chats_at_with_dependencies(
        request,
        store_path,
        messages_db_path,
        &provider,
        proposal_adapter,
    )
}

pub fn scan_selected_chats_at_with_dependencies<P, A>(
    request: ScanSelectedChatsRequest,
    store_path: &Path,
    messages_db_path: &Path,
    provider: &P,
    proposal_adapter: &A,
) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError>
where
    P: AiProvider,
    A: ProposalReplayAdapter,
{
    let source = MessagesSqliteAdapter::new(messages_db_path.to_path_buf());
    let resolved_request =
        resolve_production_scan_request(request, &source).map_err(messages_error)?;
    let recorder = NoopTraceRecorder;
    scan_selected_chats_with_dependencies(
        resolved_request,
        store_path,
        ScanSelectedChatsDependencies {
            source: &source,
            provider,
            proposal_adapter,
            trace_recorder: &recorder,
        },
    )
}

fn resolve_production_scan_request(
    request: ScanSelectedChatsRequest,
    source: &MessagesSqliteAdapter,
) -> Result<ScanSelectedChatsRequest, morrow_messages::MessagesError> {
    let chat_guids = source.resolve_public_chat_ids(&request.selected_chat_ids)?;
    Ok(request.with_native_chat_guids(&chat_guids))
}

impl ScanSelectedChatsRequest {
    fn with_native_chat_guids(mut self, chat_guids: &[ChatGuid]) -> Self {
        let raw_ids = chat_guids
            .iter()
            .map(|chat_guid| chat_guid.as_str().to_owned())
            .collect::<Vec<_>>();
        self.selected_chat_ids = raw_ids.clone();
        self.backfill_prompt_chat_ids = self
            .backfill_prompt_chat_ids
            .iter()
            .filter_map(|public_id| {
                self.selected_chats
                    .iter()
                    .position(|chat| &chat.id == public_id)
                    .and_then(|index| raw_ids.get(index).cloned())
            })
            .collect();
        for (chat, raw_id) in self.selected_chats.iter_mut().zip(raw_ids) {
            chat.id = raw_id;
        }
        self
    }
}
