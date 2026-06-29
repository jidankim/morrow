mod config;
mod dependencies;
mod messages;
mod privacy;
mod proposal_replay;

use std::{fmt::Display, path::Path};

use super::messages_sqlite::MessagesSqliteAdapter;
use config::{detection_config, reference_unix_seconds};
pub use dependencies::ScanSelectedChatsDependencies;
use dependencies::{empty_scan_result, UnavailableProvider};
use messages::ingestion_request;
use morrow_detection::{AiProvider, DetectionOutcome, DetectionPipeline};
use morrow_messages::{ingest_selected_threads, ChatGuid, IngestionStatus, MessagesDataSource};
use morrow_storage::{CapPolicy, Store};
use privacy::{candidate_ids_for_response, privacy_safe_candidate, privacy_safe_quiet_log};
use proposal_replay::{replay_external_proposals, LocalProposalAdapter};
pub use proposal_replay::{CalendarProposalReceipt, ProposalReplayAdapter};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScanSelectedChatsRequest {
    pub selected_chat_ids: Vec<String>,
    pub selected_chats: Vec<SelectedChatMetadata>,
    pub reference_timezone: String,
    pub backfill_prompt_chat_ids: Vec<String>,
    pub source_excerpts_enabled: bool,
    #[serde(default)]
    pub reference_unix_seconds: Option<i64>,
    pub cap_policy: CapPolicyRequest,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SelectedChatMetadata {
    pub id: String,
    pub participant_count: u16,
    pub participant_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum CapPolicyRequest {
    RefillForPending {
        #[serde(rename = "maxVisible")]
        max_visible: usize,
        #[serde(rename = "pendingCount")]
        pending_count: usize,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScanSelectedChatsResult {
    pub pending_proposal_count: usize,
    pub created_candidate_count: usize,
    pub quiet_log_count: usize,
    pub cap_visible_count: usize,
    pub cap_deferred_count: usize,
    pub created_external_proposal_count: usize,
    pub failed_external_proposal_count: usize,
    pub created_candidate_ids: Vec<String>,
}

#[derive(Debug)]
pub enum ScanSelectedChatsError {
    Detection(String),
    Messages(String),
    Storage(String),
    ExternalProposal(String),
}

impl Display for ScanSelectedChatsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Detection(message) => write!(formatter, "scan detection failed: {message}"),
            Self::Messages(message) => write!(formatter, "scan messages failed: {message}"),
            Self::Storage(message) => write!(formatter, "scan storage failed: {message}"),
            Self::ExternalProposal(message) => {
                write!(formatter, "scan external proposal failed: {message}")
            }
        }
    }
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
    scan_selected_chats_with_dependencies(
        resolved_request,
        store_path,
        ScanSelectedChatsDependencies {
            source: &source,
            provider,
            proposal_adapter,
        },
    )
}

pub fn scan_selected_chats_with_source(
    request: ScanSelectedChatsRequest,
    store_path: &Path,
    source: &impl MessagesDataSource,
) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError> {
    let provider = UnavailableProvider;
    let proposal_adapter = LocalProposalAdapter;
    scan_selected_chats_with_dependencies(
        request,
        store_path,
        ScanSelectedChatsDependencies {
            source,
            provider: &provider,
            proposal_adapter: &proposal_adapter,
        },
    )
}

pub fn scan_selected_chats_with_dependencies<S, P, A>(
    request: ScanSelectedChatsRequest,
    store_path: &Path,
    dependencies: ScanSelectedChatsDependencies<'_, S, P, A>,
) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError>
where
    S: MessagesDataSource,
    P: AiProvider,
    A: ProposalReplayAdapter,
{
    let store = Store::open(store_path).map_err(storage_error)?;
    let pipeline = DetectionPipeline::new(dependencies.provider);
    let reference_unix_seconds = reference_unix_seconds(&request)?;
    let ingestion = ingest_selected_threads(
        dependencies.source,
        &ingestion_request(&request, reference_unix_seconds)?,
    )
    .map_err(messages_error)?;
    if ingestion.status == IngestionStatus::Unavailable {
        return Ok(empty_scan_result());
    }
    let config = detection_config(&request, reference_unix_seconds)?;
    let report = pipeline.detect(&ingestion.messages, &config);
    let mut created_candidate_ids = Vec::new();
    let mut quiet_log_count = 0;

    for outcome in report.outcomes {
        match outcome {
            DetectionOutcome::Candidate(candidate) => {
                let candidate = privacy_safe_candidate(candidate)?;
                let candidate_id = store.create_candidate(candidate).map_err(storage_error)?;
                created_candidate_ids.push(candidate_id);
            }
            DetectionOutcome::QuietLog(quiet_log) => {
                let quiet_log = privacy_safe_quiet_log(quiet_log)?;
                store.record_quiet_log(quiet_log).map_err(storage_error)?;
                quiet_log_count += 1;
            }
        }
    }

    let cap_plan = store
        .apply_visibility_caps(request.cap_policy.into(), reference_unix_seconds)
        .map_err(storage_error)?;
    let mut replay_candidates = cap_plan.visible.clone();
    let mut recoverable_candidates = store
        .recoverable_external_proposals()
        .map_err(storage_error)?;
    recoverable_candidates.retain(|recoverable| {
        !cap_plan
            .visible
            .iter()
            .any(|visible| visible.candidate_id == recoverable.candidate_id)
    });
    replay_candidates.extend(recoverable_candidates);
    let replay =
        replay_external_proposals(&store, &replay_candidates, dependencies.proposal_adapter)?;
    let pending_proposal_count = replay_candidates.len() + cap_plan.deferred.len();
    Ok(ScanSelectedChatsResult {
        pending_proposal_count,
        created_candidate_count: created_candidate_ids.len(),
        quiet_log_count,
        cap_visible_count: cap_plan.visible.len(),
        cap_deferred_count: cap_plan.deferred.len(),
        created_external_proposal_count: replay.created,
        failed_external_proposal_count: replay.failed,
        created_candidate_ids: candidate_ids_for_response(&created_candidate_ids),
    })
}

impl From<CapPolicyRequest> for CapPolicy {
    fn from(request: CapPolicyRequest) -> Self {
        match request {
            CapPolicyRequest::RefillForPending {
                max_visible,
                pending_count,
            } => Self::refill_for_pending(max_visible, pending_count),
        }
    }
}

fn messages_error(error: morrow_messages::MessagesError) -> ScanSelectedChatsError {
    ScanSelectedChatsError::Messages(error.to_string())
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

fn storage_error(error: morrow_storage::StorageError) -> ScanSelectedChatsError {
    ScanSelectedChatsError::Storage(error.to_string())
}
