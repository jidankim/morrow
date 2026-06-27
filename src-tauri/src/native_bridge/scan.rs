mod messages;
mod proposal_replay;

use std::{fmt::Display, path::Path};

use super::messages_sqlite::MessagesSqliteAdapter;
use super::public_chat_id::{public_chat_id, public_message_id};
use messages::ingestion_request;
use morrow_detection::{
    AiProvider, ConfidenceThreshold, DetectionConfig, DetectionOutcome, DetectionPipeline,
    ProviderError, ProviderIdentity, ProviderRequest, ProviderResponse, ReferenceTime,
    SourceExcerptPolicy,
};
use morrow_messages::{ingest_selected_threads, ChatGuid, IngestionStatus, MessagesDataSource};
use morrow_storage::{CandidateDraft, CandidateId, CapPolicy, QuietLogDraft, Store};
use proposal_replay::{replay_external_proposals, LocalProposalAdapter};
use serde::{Deserialize, Serialize};

const REFERENCE_TIME: &str = "2026-06-26T09:00:00";
const NATIVE_CANDIDATE_TITLE: &str = "Messages event candidate";
const HIDDEN_SOURCE_EXCERPT: &str = "Source excerpt hidden by settings.";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScanSelectedChatsRequest {
    pub selected_chat_ids: Vec<String>,
    pub selected_chats: Vec<SelectedChatMetadata>,
    pub reference_timezone: String,
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

pub fn scan_selected_chats_at(
    request: ScanSelectedChatsRequest,
    store_path: &Path,
    messages_db_path: &Path,
) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError> {
    let source = MessagesSqliteAdapter::new(messages_db_path.to_path_buf());
    let resolved_request =
        resolve_production_scan_request(request, &source).map_err(messages_error)?;
    scan_selected_chats_with_source(resolved_request, store_path, &source)
}

pub fn scan_selected_chats_with_source(
    request: ScanSelectedChatsRequest,
    store_path: &Path,
    source: &impl MessagesDataSource,
) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError> {
    let store = Store::open(store_path).map_err(storage_error)?;
    let provider = UnavailableProvider;
    let pipeline = DetectionPipeline::new(&provider);
    let ingestion =
        ingest_selected_threads(source, &ingestion_request(&request)?).map_err(messages_error)?;
    if ingestion.status == IngestionStatus::Unavailable {
        return Ok(empty_scan_result());
    }
    let config = detection_config(&request)?;
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
        .apply_visibility_caps(request.cap_policy.into(), 1_782_352_400)
        .map_err(storage_error)?;
    let replay = replay_external_proposals(&store, &cap_plan.visible, &LocalProposalAdapter)?;
    let pending_proposal_count = cap_plan.visible.len() + cap_plan.deferred.len();
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

struct UnavailableProvider;

impl AiProvider for UnavailableProvider {
    fn extract(&self, _request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        Err(ProviderError::Unavailable {
            reason: "native scan provider is not configured".to_owned(),
        })
    }
}

fn detection_config(
    request: &ScanSelectedChatsRequest,
) -> Result<DetectionConfig, ScanSelectedChatsError> {
    Ok(DetectionConfig {
        reference: ReferenceTime::parse(REFERENCE_TIME, &request.reference_timezone)
            .map_err(detection_error)?,
        threshold: ConfidenceThreshold::new(550).map_err(detection_error)?,
        provider: ProviderIdentity::new("native-bridge", "deterministic", "scan-v1")
            .map_err(detection_error)?,
        source_excerpts: SourceExcerptPolicy::Hide,
    })
}

const fn empty_scan_result() -> ScanSelectedChatsResult {
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

fn privacy_safe_candidate(
    mut candidate: CandidateDraft,
) -> Result<CandidateDraft, ScanSelectedChatsError> {
    candidate.chat_guid =
        public_chat_id(&ChatGuid::parse(&candidate.chat_guid).map_err(messages_error)?);
    candidate.anchor_message_guid = public_message_id(&candidate.anchor_message_guid);
    candidate.title = NATIVE_CANDIDATE_TITLE.to_owned();
    candidate.evidence_excerpt = HIDDEN_SOURCE_EXCERPT.to_owned();
    Ok(candidate)
}

fn privacy_safe_quiet_log(
    mut quiet_log: QuietLogDraft,
) -> Result<QuietLogDraft, ScanSelectedChatsError> {
    quiet_log.chat_guid =
        public_chat_id(&ChatGuid::parse(&quiet_log.chat_guid).map_err(messages_error)?);
    quiet_log.anchor_message_guid = public_message_id(&quiet_log.anchor_message_guid);
    quiet_log.excerpt = HIDDEN_SOURCE_EXCERPT.to_owned();
    Ok(quiet_log)
}

fn candidate_ids_for_response(candidate_ids: &[CandidateId]) -> Vec<String> {
    candidate_ids
        .iter()
        .map(|candidate_id| candidate_id.as_str().to_owned())
        .collect()
}

fn detection_error(error: morrow_detection::DetectionError) -> ScanSelectedChatsError {
    ScanSelectedChatsError::Detection(error.to_string())
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
