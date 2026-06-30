use std::fmt::Display;

use morrow_storage::EvalRunStatus;
use serde::Serialize;

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
    pub feedback_label_count: usize,
    pub feature_snapshot_count: usize,
    pub latest_eval_status: LatestEvalStatus,
    pub created_candidate_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LatestEvalStatus {
    NeverRun,
    Passed,
    NeedsReview,
    Failed,
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

impl From<Option<EvalRunStatus>> for LatestEvalStatus {
    fn from(status: Option<EvalRunStatus>) -> Self {
        match status {
            None => Self::NeverRun,
            Some(EvalRunStatus::Passed) => Self::Passed,
            Some(EvalRunStatus::NeedsReview) => Self::NeedsReview,
            Some(EvalRunStatus::Failed) => Self::Failed,
        }
    }
}

pub(super) fn count_to_usize(
    value: i64,
    field: &'static str,
) -> Result<usize, ScanSelectedChatsError> {
    usize::try_from(value).map_err(|err| {
        ScanSelectedChatsError::Storage(format!("feedback eval {field} is out of range: {err}"))
    })
}
