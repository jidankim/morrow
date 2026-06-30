mod config;
mod dependencies;
mod feedback;
mod messages;
mod production;
mod proposal_replay;
mod result;

use std::path::Path;

use super::scan_privacy::{
    candidate_ids_for_response, privacy_safe_candidate, privacy_safe_quiet_log,
};
use config::{reference_unix_seconds, scan_config};
pub use dependencies::ScanSelectedChatsDependencies;
use dependencies::{empty_scan_result, UnavailableProvider};
use feedback::FeedbackTraceRecorder;
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
use result::count_to_usize;
pub use result::{LatestEvalStatus, ScanSelectedChatsError, ScanSelectedChatsResult};
use serde::Deserialize;

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
    pub feedback_text_snapshots_enabled: bool,
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
        return empty_scan_result(&store.feedback_eval_counts().map_err(storage_error)?);
    }
    let config = scan_config(&request, reference_unix_seconds)?;
    debug_assert!(
        !config.feedback_text_snapshots_enabled || request.source_excerpts_enabled,
        "feedback text snapshots require source excerpt consent"
    );
    let feedback_recorder = FeedbackTraceRecorder::new(dependencies.trace_recorder);
    let report =
        pipeline.detect_with_trace(&ingestion.messages, &config.detection, &feedback_recorder);
    let trace_groups = feedback_recorder.trace_groups()?;
    let mut created_candidate_ids = Vec::new();
    let mut quiet_log_count = 0;

    for (index, outcome) in report.outcomes.into_iter().enumerate() {
        let Some(message) = ingestion.messages.get(index) else {
            return Err(ScanSelectedChatsError::Detection(
                "detection outcome missing message evidence".to_owned(),
            ));
        };
        let trace_group = trace_groups.get(index);
        match outcome {
            DetectionOutcome::Candidate(candidate) => {
                let candidate =
                    privacy_safe_candidate(candidate, config.detection.source_excerpts)?;
                let candidate_id = store
                    .create_candidate(candidate.clone())
                    .map_err(storage_error)?;
                feedback::record_candidate_feedback(feedback::CandidateFeedback {
                    store: &store,
                    candidate_id: &candidate_id,
                    candidate: &candidate,
                    message,
                    trace_group,
                    config: &config,
                })?;
                created_candidate_ids.push(candidate_id);
            }
            DetectionOutcome::QuietLog(quiet_log) => {
                let quiet_log =
                    privacy_safe_quiet_log(quiet_log, config.detection.source_excerpts)?;
                store
                    .record_quiet_log(quiet_log.clone())
                    .map_err(storage_error)?;
                feedback::record_quiet_feedback(feedback::QuietFeedback {
                    store: &store,
                    quiet_log: &quiet_log,
                    message,
                    trace_group,
                    config: &config,
                })?;
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
    let feedback_eval_counts = store.feedback_eval_counts().map_err(storage_error)?;
    Ok(ScanSelectedChatsResult {
        pending_proposal_count,
        created_candidate_count: created_candidate_ids.len(),
        quiet_log_count,
        cap_visible_count: cap_plan.visible.len(),
        cap_deferred_count: cap_plan.deferred.len(),
        created_external_proposal_count: replay.created,
        failed_external_proposal_count: replay.failed,
        feedback_label_count: count_to_usize(feedback_eval_counts.label_count, "label_count")?,
        feature_snapshot_count: count_to_usize(
            feedback_eval_counts.feature_snapshot_count,
            "feature_snapshot_count",
        )?,
        latest_eval_status: LatestEvalStatus::from(feedback_eval_counts.latest_eval_status),
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
