mod config;
mod dependencies;
mod feedback;
mod list_intake;
mod list_intake_execution;
mod messages;
mod outcome_plan;
mod persistence;
mod production;
mod proposal_replay;
mod provider_route_cache;
mod replay_selection;
mod result;

use std::path::Path;

use super::scan_privacy::candidate_ids_for_response;
use config::{reference_unix_seconds, scan_config};
use dependencies::empty_scan_result;
pub use dependencies::ScanSelectedChatsDependencies;
use feedback::FeedbackTraceRecorder;
pub use list_intake::ListIntakeProfileRequest;
use list_intake_execution::{run_list_intake, ListIntakeRunRequest};
use messages::ingestion_request;
use morrow_detection::{AiProvider, DetectionPipeline};
use morrow_diagnostics::TraceRecorder;
use morrow_messages::{ingest_selected_threads, IngestionStatus, MessagesDataSource};
use morrow_storage::{CapPolicy, Store};
use outcome_plan::{plan_scan_outcomes, ScanOutcomePlanRequest};
use persistence::{
    apply_scan_persistence, CandidatePersistenceFeedback, QuietLogPersistenceFeedback,
    ScanPersistenceFeedback, ScanPersistenceRequest,
};
pub use production::{
    scan_selected_chats_at_with_dependencies,
    scan_selected_chats_at_with_dependencies_and_app_data_dir,
    scan_selected_chats_at_with_unavailable_provider,
    scan_selected_chats_at_with_unavailable_provider_and_app_data_dir,
    scan_selected_chats_with_source,
};
use proposal_replay::replay_external_proposals;
pub(in crate::native_bridge) use proposal_replay::LocalProposalAdapter;
pub use proposal_replay::{
    CalendarProposalReceipt, ProposalReplayAdapter, ReminderDueComponents, ReminderDueTimeZone,
    ReminderProposalReceipt,
};
use provider_route_cache::{
    record_provider_route_ledger_writes, stage_provider_route_ledger_writes,
    NativeProviderRouteCache,
};
use replay_selection::{select_replay_candidates, ReplaySelectionInput};
use result::count_to_usize;
pub use result::{LatestEvalStatus, ScanSelectedChatsError, ScanSelectedChatsResult};
use serde::Deserialize;

pub use morrow_detection::{
    ListReminderDefaultDueMode, ListReminderDefaultDueTime, ListReminderItemOutputMode,
    ListReminderProfile, ListReminderProfileId, ListReminderProfileVersion,
    ListReminderRecurrenceMode, ListReminderRoutingMode,
};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ScanSelectedChatsRequest {
    pub selected_chat_ids: Vec<String>,
    pub selected_chats: Vec<SelectedChatMetadata>,
    pub reference_timezone: String,
    #[serde(default)]
    pub reference_unix_seconds: Option<i64>,
    pub backfill_prompt_chat_ids: Vec<String>,
    pub source_excerpts_enabled: bool,
    pub feedback_text_snapshots_enabled: bool,
    pub local_diagnostics_enabled: bool,
    pub local_diagnostics_retention_days: u16,
    #[serde(deserialize_with = "list_intake::deserialize_profiles")]
    pub list_intake_profiles: Vec<ListIntakeProfileRequest>,
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
    request.validate_local_diagnostics_retention()?;
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
    let provider_route_cache = NativeProviderRouteCache::new(&store);
    let report = pipeline
        .detect_with_trace_and_provider_cache(
            &ingestion.messages,
            &config.detection,
            &feedback_recorder,
            &provider_route_cache,
        )
        .map_err(provider_route_cache::pipeline_error)?;
    let trace_groups = feedback_recorder.trace_groups()?;
    let morrow_detection::DetectionReport {
        outcomes,
        provider_route_write_intents,
    } = report;
    run_list_intake(ListIntakeRunRequest {
        store: &store,
        provider: dependencies.provider,
        profiles: &request.list_intake_profiles,
        messages: &ingestion.messages,
        sender_groups: &ingestion.sender_groups,
        outcomes: &outcomes,
        reference_timezone: &config.detection.reference.timezone,
    })?;
    let outcome_plan = plan_scan_outcomes(ScanOutcomePlanRequest {
        outcomes,
        messages: &ingestion.messages,
        trace_group_count: trace_groups.len(),
        source_excerpts: config.detection.source_excerpts,
    })?;
    let provider_route_writes =
        stage_provider_route_ledger_writes(&outcome_plan.intents, &provider_route_write_intents)?;
    let persistence = apply_scan_persistence(ScanPersistenceRequest {
        store: &store,
        intents: outcome_plan.intents,
        feedback: ScanPersistenceFeedback {
            record_candidate: |input: CandidatePersistenceFeedback<'_, '_>| {
                let trace_group = input
                    .planned
                    .trace_group_index
                    .and_then(|index| trace_groups.get(index));
                feedback::record_candidate_feedback(feedback::CandidateFeedback {
                    store: &store,
                    candidate_id: input.candidate_id,
                    candidate: &input.planned.candidate,
                    message: input.planned.message,
                    trace_group,
                    config: &config,
                })
            },
            record_quiet_log: |input: QuietLogPersistenceFeedback<'_, '_>| {
                let trace_group = input
                    .planned
                    .trace_group_index
                    .and_then(|index| trace_groups.get(index));
                feedback::record_quiet_feedback(feedback::QuietFeedback {
                    store: &store,
                    quiet_log: &input.planned.quiet_log,
                    message: input.planned.message,
                    trace_group,
                    config: &config,
                })
            },
        },
    })?;

    let cap_plan = store
        .apply_visibility_caps(request.cap_policy.into(), 1_782_352_400)
        .map_err(storage_error)?;
    let recoverable_candidates = store
        .recoverable_external_proposals()
        .map_err(storage_error)?;
    let replay_selection = select_replay_candidates(ReplaySelectionInput {
        visible: &cap_plan.visible,
        deferred: &cap_plan.deferred,
        recoverable: recoverable_candidates,
    });
    let replay = replay_external_proposals(
        &store,
        &replay_selection.replay_candidates,
        dependencies.proposal_adapter,
    )?;
    if replay.failed == 0 {
        record_provider_route_ledger_writes(&store, provider_route_writes)?;
    }
    let feedback_eval_counts = store.feedback_eval_counts().map_err(storage_error)?;
    Ok(ScanSelectedChatsResult {
        pending_proposal_count: replay_selection.pending_proposal_count,
        created_candidate_count: persistence.created_candidate_ids.len(),
        quiet_log_count: persistence.quiet_log_count,
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
        created_candidate_ids: candidate_ids_for_response(&persistence.created_candidate_ids),
    })
}

impl ScanSelectedChatsRequest {
    pub(in crate::native_bridge) fn validate_local_diagnostics_retention(
        &self,
    ) -> Result<(), ScanSelectedChatsError> {
        config::validate_local_diagnostics_retention(self)
    }
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
