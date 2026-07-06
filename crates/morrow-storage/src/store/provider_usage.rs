mod aggregation;

use crate::provider_usage::{ProviderUsageReport, ProviderUsageWindow, ProviderUsageWindowKey};
use crate::sqlite_cli::{row_value, SqliteValue};
use crate::validation::validate_text;
use crate::StorageError;

use super::Store;

#[derive(Debug, Clone)]
pub(super) struct ProviderUsageRow {
    id: i64,
    provider_id: String,
    model_id: String,
    prompt_version: String,
    outcome_kind: ProviderUsageOutcomeKind,
    route_label: String,
    confidence: Option<f64>,
    created_at_unix_seconds: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProviderUsageOutcomeKind {
    Candidate,
    Quiet,
}

impl ProviderUsageOutcomeKind {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Quiet => "quiet",
        }
    }
}

impl Store {
    pub fn provider_usage_report(
        &self,
        window_key: ProviderUsageWindowKey,
        now_unix_seconds: i64,
    ) -> Result<ProviderUsageReport, StorageError> {
        let window = ProviderUsageWindow {
            key: window_key,
            start_unix_seconds: window_key.start_unix_seconds(now_unix_seconds),
            end_unix_seconds: now_unix_seconds,
        };
        let rows = self.provider_usage_rows(&window)?;
        aggregation::build_report(now_unix_seconds, window, &rows)
    }

    fn provider_usage_rows(
        &self,
        window: &ProviderUsageWindow,
    ) -> Result<Vec<ProviderUsageRow>, StorageError> {
        let bounded_sql =
            "SELECT id, provider_id, model_id, prompt_version, outcome_kind, parser_route,
                    candidate_confidence_millis, created_at
             FROM provider_route_outcomes
             WHERE created_at >= :p0 AND created_at <= :p1
             ORDER BY id ASC;";
        let all_sql =
            "SELECT id, provider_id, model_id, prompt_version, outcome_kind, parser_route,
                    candidate_confidence_millis, created_at
             FROM provider_route_outcomes
             WHERE created_at <= :p0
             ORDER BY id ASC;";
        let rows = match window.start_unix_seconds {
            Some(start) => self.sqlite.query_rows_with_params(
                bounded_sql,
                &[
                    SqliteValue::Integer(start),
                    SqliteValue::Integer(window.end_unix_seconds),
                ],
            )?,
            None => self.sqlite.query_rows_with_params(
                all_sql,
                &[SqliteValue::Integer(window.end_unix_seconds)],
            )?,
        };
        Ok(rows
            .iter()
            .filter_map(|row| parse_provider_usage_row(row))
            .collect())
    }
}

fn parse_provider_usage_row(row: &[String]) -> Option<ProviderUsageRow> {
    let id = parse_i64(row_value(row, 0, "provider_usage.id").ok()?)?;
    let provider_id = report_text(row_value(row, 1, "provider_usage.provider_id").ok()?, 80)?;
    let model_id = report_text(row_value(row, 2, "provider_usage.model_id").ok()?, 120)?;
    let prompt_version = report_text(
        row_value(row, 3, "provider_usage.prompt_version").ok()?,
        120,
    )?;
    let outcome_kind = parse_outcome_kind(row_value(row, 4, "provider_usage.outcome_kind").ok()?)?;
    let parser_route = report_text(row_value(row, 5, "provider_usage.route_label").ok()?, 180)?;
    let route_label = public_route_label(&parser_route).to_owned();
    let confidence = parse_confidence(row_value(row, 6, "provider_usage.confidence").ok()?)?;
    let created_at_unix_seconds = parse_i64(row_value(row, 7, "provider_usage.created_at").ok()?)?;
    Some(ProviderUsageRow {
        id,
        provider_id,
        model_id,
        prompt_version,
        outcome_kind,
        route_label,
        confidence,
        created_at_unix_seconds,
    })
}

fn report_text(raw: &str, max_len: usize) -> Option<String> {
    validate_text("provider_usage_report", raw, max_len)
        .ok()
        .map(|()| raw.to_owned())
}

fn public_route_label(raw: &str) -> &'static str {
    if raw == "parser_time" || raw.starts_with("parser_time:") {
        return "parser_time";
    }
    if raw == "provider_candidate" || raw == "provider_route" || raw == "candidate_route" {
        return "provider_candidate";
    }
    if raw == "provider_rejection"
        || raw == "provider_rejected"
        || raw.starts_with("provider_rejected:")
        || raw == "provider_unavailable"
    {
        return "provider_rejection";
    }
    if raw == "parser_pattern"
        || raw == "calendar_route"
        || raw == "quiet_route"
        || raw == "no_action"
        || raw.starts_with("parser_provider_route_")
        || raw.starts_with("parser_pattern:")
    {
        return "parser_pattern";
    }
    "unknown"
}

fn parse_outcome_kind(raw: &str) -> Option<ProviderUsageOutcomeKind> {
    match raw {
        "candidate" => Some(ProviderUsageOutcomeKind::Candidate),
        "quiet" => Some(ProviderUsageOutcomeKind::Quiet),
        _ => None,
    }
}

fn parse_i64(raw: &str) -> Option<i64> {
    raw.parse::<i64>().ok()
}

fn parse_confidence(raw: &str) -> Option<Option<f64>> {
    if raw.is_empty() {
        return Some(None);
    }
    raw.parse::<i64>()
        .ok()
        .filter(|confidence| (0..=1000).contains(confidence))
        .and_then(|confidence| u16::try_from(confidence).ok())
        .map(|confidence| Some(f64::from(confidence) / 1000.0))
}
