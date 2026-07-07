use std::{path::Path, time::Duration};

use super::dependencies::UnavailableProvider;
use super::{
    list_intake, messages_error, scan_selected_chats_with_dependencies, LocalProposalAdapter,
    ProposalReplayAdapter, ScanSelectedChatsDependencies, ScanSelectedChatsError,
    ScanSelectedChatsRequest, ScanSelectedChatsResult,
};
use crate::native_bridge::messages_sqlite::MessagesSqliteAdapter;
use morrow_detection::AiProvider;
use morrow_diagnostics::{
    JsonlTraceSink, JsonlTraceSinkConfig, NoopTraceRecorder, TraceRecord, TraceRecorder,
    TraceRecorderError,
};
use morrow_messages::{ChatGuid, MessagesDataSource};
use morrow_storage::Store;

const TRACE_ROTATION_BYTES: u64 = 10 * 1024 * 1024;
const SECONDS_PER_DAY: u64 = 86_400;

pub fn scan_selected_chats_with_source(
    request: ScanSelectedChatsRequest,
    store_path: &Path,
    source: &impl MessagesDataSource,
) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError> {
    let recorder = NoopTraceRecorder;
    let provider = UnavailableProvider;
    let proposal_adapter = LocalProposalAdapter;
    scan_selected_chats_with_dependencies(
        request,
        store_path,
        ScanSelectedChatsDependencies {
            source,
            provider: &provider,
            proposal_adapter: &proposal_adapter,
            trace_recorder: &recorder,
        },
    )
}

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

pub fn scan_selected_chats_at_with_unavailable_provider_and_app_data_dir<A>(
    request: ScanSelectedChatsRequest,
    store_path: &Path,
    messages_db_path: &Path,
    app_data_dir: &Path,
    proposal_adapter: &A,
) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError>
where
    A: ProposalReplayAdapter,
{
    let provider = UnavailableProvider;
    scan_selected_chats_at_with_dependencies_and_app_data_dir(
        request,
        store_path,
        messages_db_path,
        app_data_dir,
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
    request.validate_local_diagnostics_retention()?;
    let recorder = NoopTraceRecorder;
    scan_selected_chats_at_with_recorder(
        request,
        store_path,
        messages_db_path,
        provider,
        proposal_adapter,
        &recorder,
    )
}

pub fn scan_selected_chats_at_with_dependencies_and_app_data_dir<P, A>(
    request: ScanSelectedChatsRequest,
    store_path: &Path,
    messages_db_path: &Path,
    app_data_dir: &Path,
    provider: &P,
    proposal_adapter: &A,
) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError>
where
    P: AiProvider,
    A: ProposalReplayAdapter,
{
    request.validate_local_diagnostics_retention()?;
    let recorder = select_production_trace_recorder(&request, app_data_dir);
    scan_selected_chats_at_with_recorder(
        request,
        store_path,
        messages_db_path,
        provider,
        proposal_adapter,
        &recorder,
    )
}

fn scan_selected_chats_at_with_recorder<P, A, R>(
    request: ScanSelectedChatsRequest,
    store_path: &Path,
    messages_db_path: &Path,
    provider: &P,
    proposal_adapter: &A,
    trace_recorder: &R,
) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError>
where
    P: AiProvider,
    A: ProposalReplayAdapter,
    R: TraceRecorder + ?Sized,
{
    let store = Store::open(store_path).map_err(super::storage_error)?;
    let source = MessagesSqliteAdapter::with_local_store(messages_db_path.to_path_buf(), &store)
        .map_err(messages_error)?;
    let resolved_request =
        resolve_production_scan_request(request, &source).map_err(messages_error)?;
    scan_selected_chats_with_dependencies(
        resolved_request,
        store_path,
        ScanSelectedChatsDependencies {
            source: &source,
            provider,
            proposal_adapter,
            trace_recorder,
        },
    )
}

enum ProductionTraceRecorder {
    Noop(NoopTraceRecorder),
    Jsonl(JsonlTraceSink),
}

impl TraceRecorder for ProductionTraceRecorder {
    fn record(&self, record: &TraceRecord) -> Result<(), TraceRecorderError> {
        match self {
            Self::Noop(recorder) => recorder.record(record),
            Self::Jsonl(recorder) => recorder.record(record),
        }
    }
}

fn select_production_trace_recorder(
    request: &ScanSelectedChatsRequest,
    app_data_dir: &Path,
) -> ProductionTraceRecorder {
    if request.local_diagnostics_enabled {
        let retention = Duration::from_secs(
            u64::from(request.local_diagnostics_retention_days) * SECONDS_PER_DAY,
        );
        let config = JsonlTraceSinkConfig::new(retention, TRACE_ROTATION_BYTES);
        match JsonlTraceSink::with_config(app_data_dir, config) {
            Ok(sink) => ProductionTraceRecorder::Jsonl(sink),
            Err(_error) => ProductionTraceRecorder::Noop(NoopTraceRecorder),
        }
    } else {
        ProductionTraceRecorder::Noop(NoopTraceRecorder)
    }
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
        let public_ids = self.selected_chat_ids.clone();
        let raw_ids = chat_guids
            .iter()
            .map(|chat_guid| chat_guid.as_str().to_owned())
            .collect::<Vec<_>>();
        list_intake::rewrite_selected_chat_scope_ids(
            &mut self.list_intake_profiles,
            &public_ids,
            &raw_ids,
        );
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
