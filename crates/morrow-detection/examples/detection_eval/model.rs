use std::collections::BTreeMap;

use morrow_diagnostics::{TraceRecord, TraceSchemaVersion};
use serde::Serialize;
use serde_json::Value;

pub struct EvalRun {
    pub summary: EvalSummary,
    pub records: Vec<TraceRecord>,
    pub passed: bool,
}

#[derive(Serialize)]
pub struct EvalSummary {
    pub dataset_version: String,
    pub dataset_family: String,
    pub trace_schema_version: TraceSchemaVersion,
    pub provider_identity: ProviderSummary,
    pub scenario_totals: ScenarioTotals,
    pub scenarios: Vec<ScenarioSummary>,
    pub false_positive_count: usize,
    pub false_negative_count: usize,
    pub route_reason_confusion_counts: BTreeMap<String, usize>,
    pub unsafe_action_count: usize,
    pub privacy_leakage_count: usize,
    pub prompt_injection_resilience_count: usize,
    pub canary_scan_result: CanaryScanResult,
    pub skipped_corrupt_line_count: usize,
    pub mismatch_count: usize,
}

#[derive(Serialize)]
pub struct ScenarioTotals {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
}

#[derive(Serialize)]
pub struct ProviderSummary {
    pub provider_id: String,
    pub model_id: String,
    pub template_version: String,
}

#[derive(Serialize)]
pub struct ScenarioSummary {
    pub name: String,
    pub passed: bool,
    pub candidate_count: usize,
    pub quiet_count: usize,
    pub provider_call_count: usize,
    pub false_positive: bool,
    pub false_negative: bool,
    pub unsafe_action: bool,
    pub privacy_leakage: bool,
    pub prompt_injection_resilient: bool,
    pub mismatches: Vec<Mismatch>,
}

#[derive(Serialize)]
pub struct CanaryScanResult {
    pub passed: bool,
    pub scanned_artifacts: [&'static str; 2],
    pub failed_artifacts: Vec<&'static str>,
}

#[derive(Serialize)]
pub struct Mismatch {
    pub step: Option<usize>,
    pub field: &'static str,
    pub expected: Value,
    pub actual: Value,
}

pub struct ScenarioRun {
    pub records: Vec<TraceRecord>,
    pub summary: ScenarioSummary,
}
