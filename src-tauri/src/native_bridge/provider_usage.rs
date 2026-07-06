use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use morrow_storage::{
    ProviderUsageModel as StorageProviderUsageModel,
    ProviderUsagePromptVersion as StorageProviderUsagePromptVersion,
    ProviderUsageProvider as StorageProviderUsageProvider,
    ProviderUsageRecentOutcome as StorageProviderUsageRecentOutcome,
    ProviderUsageReport as StorageProviderUsageReport,
    ProviderUsageTotals as StorageProviderUsageTotals,
    ProviderUsageWindow as StorageProviderUsageWindow, ProviderUsageWindowKey, Store,
};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use super::paths::morrow_store_path;

const PROVIDER_USAGE_STORE_UNAVAILABLE_ERROR: &str = "provider usage store unavailable";
const INVALID_PROVIDER_USAGE_WINDOW_ERROR: &str = "invalid provider usage window";

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LoadProviderUsageRequest {
    #[serde(default)]
    pub window_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsageCommandReport {
    pub generated_at_unix_seconds: i64,
    pub window: ProviderUsageCommandWindow,
    pub totals: ProviderUsageCommandTotals,
    pub providers: Vec<ProviderUsageCommandProvider>,
    pub recent_outcomes: Vec<ProviderUsageCommandRecentOutcome>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsageCommandWindow {
    pub key: String,
    pub start_unix_seconds: Option<i64>,
    pub end_unix_seconds: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsageCommandTotals {
    pub total_outcomes: u32,
    pub candidate_count: u32,
    pub quiet_count: u32,
    pub quiet_rate: f64,
    pub candidate_rate: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsageCommandProvider {
    pub provider_id: String,
    pub total_outcomes: u32,
    pub candidate_count: u32,
    pub quiet_count: u32,
    pub quiet_rate: f64,
    pub candidate_rate: f64,
    pub average_confidence: Option<f64>,
    pub models: Vec<ProviderUsageCommandModel>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsageCommandModel {
    pub model_id: String,
    pub total_outcomes: u32,
    pub candidate_count: u32,
    pub quiet_count: u32,
    pub quiet_rate: f64,
    pub candidate_rate: f64,
    pub average_confidence: Option<f64>,
    pub prompt_versions: Vec<ProviderUsageCommandPromptVersion>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsageCommandPromptVersion {
    pub prompt_version: String,
    pub total_outcomes: u32,
    pub candidate_count: u32,
    pub quiet_count: u32,
    pub quiet_rate: f64,
    pub candidate_rate: f64,
    pub average_confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsageCommandRecentOutcome {
    pub provider_id: String,
    pub model_id: String,
    pub prompt_version: String,
    pub outcome_kind: String,
    pub route_label: String,
    pub confidence: Option<f64>,
    pub created_at_unix_seconds: i64,
}

pub fn current_unix_seconds() -> Result<i64, String> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| provider_usage_store_unavailable())?;
    i64::try_from(duration.as_secs()).map_err(|_| provider_usage_store_unavailable())
}

#[tauri::command]
pub fn load_provider_usage(
    app: AppHandle,
    request: LoadProviderUsageRequest,
) -> Result<ProviderUsageCommandReport, String> {
    let store_path = morrow_store_path(&app).map_err(|_| provider_usage_store_unavailable())?;
    let now_unix_seconds = current_unix_seconds()?;
    load_provider_usage_at(&store_path, request, now_unix_seconds)
}

pub fn load_provider_usage_at(
    store_path: &Path,
    request: LoadProviderUsageRequest,
    now_unix_seconds: i64,
) -> Result<ProviderUsageCommandReport, String> {
    let window_key = parse_window_key(request.window_key.as_deref())?;
    let store = Store::open(store_path).map_err(|_| provider_usage_store_unavailable())?;
    let report = store
        .provider_usage_report(window_key, now_unix_seconds)
        .map_err(|_| provider_usage_store_unavailable())?;
    Ok(ProviderUsageCommandReport::from(report))
}

fn parse_window_key(raw: Option<&str>) -> Result<ProviderUsageWindowKey, String> {
    match raw {
        None => Ok(ProviderUsageWindowKey::ThirtyDays),
        Some("7d") => Ok(ProviderUsageWindowKey::SevenDays),
        Some("30d") => Ok(ProviderUsageWindowKey::ThirtyDays),
        Some("90d") => Ok(ProviderUsageWindowKey::NinetyDays),
        Some("all") => Ok(ProviderUsageWindowKey::All),
        Some(_) => Err(INVALID_PROVIDER_USAGE_WINDOW_ERROR.to_owned()),
    }
}

fn provider_usage_store_unavailable() -> String {
    PROVIDER_USAGE_STORE_UNAVAILABLE_ERROR.to_owned()
}

impl From<StorageProviderUsageReport> for ProviderUsageCommandReport {
    fn from(report: StorageProviderUsageReport) -> Self {
        Self {
            generated_at_unix_seconds: report.generated_at_unix_seconds,
            window: ProviderUsageCommandWindow::from(report.window),
            totals: ProviderUsageCommandTotals::from(report.totals),
            providers: report
                .providers
                .into_iter()
                .map(ProviderUsageCommandProvider::from)
                .collect(),
            recent_outcomes: report
                .recent_outcomes
                .into_iter()
                .map(ProviderUsageCommandRecentOutcome::from)
                .collect(),
        }
    }
}

impl From<StorageProviderUsageWindow> for ProviderUsageCommandWindow {
    fn from(window: StorageProviderUsageWindow) -> Self {
        Self {
            key: window.key.as_str().to_owned(),
            start_unix_seconds: window.start_unix_seconds,
            end_unix_seconds: window.end_unix_seconds,
        }
    }
}

impl From<StorageProviderUsageTotals> for ProviderUsageCommandTotals {
    fn from(totals: StorageProviderUsageTotals) -> Self {
        Self {
            total_outcomes: totals.total_outcomes,
            candidate_count: totals.candidate_count,
            quiet_count: totals.quiet_count,
            quiet_rate: totals.quiet_rate,
            candidate_rate: totals.candidate_rate,
        }
    }
}

impl From<StorageProviderUsageProvider> for ProviderUsageCommandProvider {
    fn from(provider: StorageProviderUsageProvider) -> Self {
        Self {
            provider_id: provider.provider_id,
            total_outcomes: provider.total_outcomes,
            candidate_count: provider.candidate_count,
            quiet_count: provider.quiet_count,
            quiet_rate: provider.quiet_rate,
            candidate_rate: provider.candidate_rate,
            average_confidence: provider.average_confidence,
            models: provider
                .models
                .into_iter()
                .map(ProviderUsageCommandModel::from)
                .collect(),
        }
    }
}

impl From<StorageProviderUsageModel> for ProviderUsageCommandModel {
    fn from(model: StorageProviderUsageModel) -> Self {
        Self {
            model_id: model.model_id,
            total_outcomes: model.total_outcomes,
            candidate_count: model.candidate_count,
            quiet_count: model.quiet_count,
            quiet_rate: model.quiet_rate,
            candidate_rate: model.candidate_rate,
            average_confidence: model.average_confidence,
            prompt_versions: model
                .prompt_versions
                .into_iter()
                .map(ProviderUsageCommandPromptVersion::from)
                .collect(),
        }
    }
}

impl From<StorageProviderUsagePromptVersion> for ProviderUsageCommandPromptVersion {
    fn from(prompt_version: StorageProviderUsagePromptVersion) -> Self {
        Self {
            prompt_version: prompt_version.prompt_version,
            total_outcomes: prompt_version.total_outcomes,
            candidate_count: prompt_version.candidate_count,
            quiet_count: prompt_version.quiet_count,
            quiet_rate: prompt_version.quiet_rate,
            candidate_rate: prompt_version.candidate_rate,
            average_confidence: prompt_version.average_confidence,
        }
    }
}

impl From<StorageProviderUsageRecentOutcome> for ProviderUsageCommandRecentOutcome {
    fn from(outcome: StorageProviderUsageRecentOutcome) -> Self {
        Self {
            provider_id: outcome.provider_id,
            model_id: outcome.model_id,
            prompt_version: outcome.prompt_version,
            outcome_kind: outcome.outcome_kind,
            route_label: outcome.route_label,
            confidence: outcome.confidence,
            created_at_unix_seconds: outcome.created_at_unix_seconds,
        }
    }
}
