mod config;
mod dependencies;
mod messages;
mod production;
mod proposal_replay;

use std::{fmt::Display, path::Path};

use super::scan_privacy::{
    candidate_ids_for_response, privacy_safe_candidate, privacy_safe_quiet_log,
};
use config::{detection_config, reference_unix_seconds};
pub use dependencies::ScanSelectedChatsDependencies;
use dependencies::{empty_scan_result, UnavailableProvider};
use messages::ingestion_request;
use morrow_detection::{AiProvider, DetectionOutcome, DetectionPipeline};
use morrow_diagnostics::{NoopTraceRecorder, TraceRecorder};
use morrow_messages::{ingest_selected_threads, IngestionStatus, MessagesDataSource};
use morrow_storage::{CapPolicy, Store};
pub use production::{
    scan_selected_chats_at_with_dependencies, scan_selected_chats_at_with_unavailable_provider,
};
use proposal_replay::replay_external_proposals;
pub(in crate::native_bridge) use proposal_replay::LocalProposalAdapter;
pub use proposal_replay::{CalendarProposalReceipt, ProposalReplayAdapter};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScanSelectedChatsRequest {
    pub selected_chat_ids: Vec<String>,
    pub selected_chats: Vec<SelectedChatMetadata>,
    pub reference_timezone: String,
    #[serde(default)]
    pub reference_unix_seconds: Option<i64>,
    pub backfill_prompt_chat_ids: Vec<String>,
    pub source_excerpts_enabled: bool,
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

pub fn scan_selected_chats_with_dependencies<S, P, A, R>(
    request: ScanSelectedChatsRequest,
    store_path: &Path,
    dependencies: ScanSelectedChatsDependencies<'_, S, P, A, R>,
) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError>
where
    S: MessagesDataSource,
    P: AiProvider,
    A: ProposalReplayAdapter,
    R: TraceRecorder + ?Sized,
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
    let report =
        pipeline.detect_with_trace(&ingestion.messages, &config, dependencies.trace_recorder);
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
        .apply_visibility_caps(request.cap_policy.into(), 1_782_352_400)
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

fn storage_error(error: morrow_storage::StorageError) -> ScanSelectedChatsError {
    ScanSelectedChatsError::Storage(error.to_string())
}
