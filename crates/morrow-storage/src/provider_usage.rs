pub const PROVIDER_USAGE_RECENT_OUTCOME_LIMIT: usize = 25;

const DAY_SECONDS: i64 = 24 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderUsageWindowKey {
    SevenDays,
    ThirtyDays,
    NinetyDays,
    All,
}

impl ProviderUsageWindowKey {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SevenDays => "7d",
            Self::ThirtyDays => "30d",
            Self::NinetyDays => "90d",
            Self::All => "all",
        }
    }

    pub(crate) const fn start_unix_seconds(self, now_unix_seconds: i64) -> Option<i64> {
        match self {
            Self::SevenDays => Some(now_unix_seconds - (7 * DAY_SECONDS)),
            Self::ThirtyDays => Some(now_unix_seconds - (30 * DAY_SECONDS)),
            Self::NinetyDays => Some(now_unix_seconds - (90 * DAY_SECONDS)),
            Self::All => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProviderUsageReport {
    pub generated_at_unix_seconds: i64,
    pub window: ProviderUsageWindow,
    pub totals: ProviderUsageTotals,
    pub providers: Vec<ProviderUsageProvider>,
    pub recent_outcomes: Vec<ProviderUsageRecentOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderUsageWindow {
    pub key: ProviderUsageWindowKey,
    pub start_unix_seconds: Option<i64>,
    pub end_unix_seconds: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProviderUsageTotals {
    pub total_outcomes: u32,
    pub candidate_count: u32,
    pub quiet_count: u32,
    pub quiet_rate: f64,
    pub candidate_rate: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProviderUsageProvider {
    pub provider_id: String,
    pub total_outcomes: u32,
    pub candidate_count: u32,
    pub quiet_count: u32,
    pub quiet_rate: f64,
    pub candidate_rate: f64,
    pub average_confidence: Option<f64>,
    pub models: Vec<ProviderUsageModel>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProviderUsageModel {
    pub model_id: String,
    pub total_outcomes: u32,
    pub candidate_count: u32,
    pub quiet_count: u32,
    pub quiet_rate: f64,
    pub candidate_rate: f64,
    pub average_confidence: Option<f64>,
    pub prompt_versions: Vec<ProviderUsagePromptVersion>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProviderUsagePromptVersion {
    pub prompt_version: String,
    pub total_outcomes: u32,
    pub candidate_count: u32,
    pub quiet_count: u32,
    pub quiet_rate: f64,
    pub candidate_rate: f64,
    pub average_confidence: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProviderUsageRecentOutcome {
    pub provider_id: String,
    pub model_id: String,
    pub prompt_version: String,
    pub outcome_kind: String,
    pub route_label: String,
    pub confidence: Option<f64>,
    pub created_at_unix_seconds: i64,
}
