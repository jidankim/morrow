use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use super::paths::morrow_store_path;

mod labels;
mod report;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListIntakeReviewReport {
    pub generated_at_unix_seconds: i64,
    pub aggregates: Vec<ListIntakeAggregateGroup>,
    pub proposals: Vec<ListIntakeReviewProposalCommand>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListIntakeAggregateGroup {
    pub profile_id: String,
    pub profile_name: String,
    pub output_policy: String,
    pub local_date: String,
    pub chat_label: String,
    pub sender_label: String,
    pub category_label: String,
    pub items: Vec<ListIntakeReviewItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListIntakeReviewItem {
    pub item_name: String,
    pub quantity: i64,
    pub unit: Option<String>,
    pub category_id: String,
    pub category_label: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListIntakeReviewProposalCommand {
    pub proposal_id: String,
    pub profile_name: String,
    pub chat_label: String,
    pub sender_label: String,
    pub items: Vec<ListIntakeReviewItem>,
    pub categories: Vec<ListIntakeReviewCategory>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListIntakeReviewCategory {
    pub category_id: String,
    pub label: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "decision")]
pub enum ListIntakeDecisionRequest {
    #[serde(rename = "approveEdited", rename_all = "camelCase")]
    ApproveEdited {
        proposal_id: String,
        items: Vec<ListIntakeReviewItem>,
    },
    #[serde(rename = "reject", rename_all = "camelCase")]
    Reject { proposal_id: String },
}

#[tauri::command]
pub fn load_list_intake_review(app: AppHandle) -> Result<ListIntakeReviewReport, String> {
    let store_path = morrow_store_path(&app)?;
    load_list_intake_review_at(&store_path, now_unix_seconds()?)
}

#[tauri::command]
pub fn decide_list_intake_proposal(
    app: AppHandle,
    request: ListIntakeDecisionRequest,
) -> Result<ListIntakeReviewReport, String> {
    let store_path = morrow_store_path(&app)?;
    let now = now_unix_seconds()?;
    decide_list_intake_proposal_at(&store_path, request, now, now)
}

pub fn load_list_intake_review_at(
    store_path: &Path,
    generated_at_unix_seconds: i64,
) -> Result<ListIntakeReviewReport, String> {
    report::load_at(store_path, generated_at_unix_seconds)
}

pub fn decide_list_intake_proposal_at(
    store_path: &Path,
    request: ListIntakeDecisionRequest,
    decided_at: i64,
    generated_at_unix_seconds: i64,
) -> Result<ListIntakeReviewReport, String> {
    report::decide_at(store_path, request, decided_at, generated_at_unix_seconds)
}

fn now_unix_seconds() -> Result<i64, String> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?;
    i64::try_from(duration.as_secs()).map_err(|error| error.to_string())
}
