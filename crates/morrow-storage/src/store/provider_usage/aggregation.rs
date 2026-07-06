use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::provider_usage::{
    ProviderUsageModel, ProviderUsagePromptVersion, ProviderUsageProvider,
    ProviderUsageRecentOutcome, ProviderUsageReport, ProviderUsageTotals, ProviderUsageWindow,
    PROVIDER_USAGE_RECENT_OUTCOME_LIMIT,
};
use crate::StorageError;

use super::{ProviderUsageOutcomeKind, ProviderUsageRow};

#[derive(Debug, Default, Clone)]
struct CountAccumulator {
    total_outcomes: u32,
    candidate_count: u32,
    quiet_count: u32,
    confidence_sum: f64,
    confidence_count: u32,
}

impl CountAccumulator {
    fn add(&mut self, row: &ProviderUsageRow) -> Result<(), StorageError> {
        self.total_outcomes = increment_count(self.total_outcomes, "total_outcomes")?;
        match row.outcome_kind {
            ProviderUsageOutcomeKind::Candidate => {
                self.candidate_count = increment_count(self.candidate_count, "candidate_count")?;
            }
            ProviderUsageOutcomeKind::Quiet => {
                self.quiet_count = increment_count(self.quiet_count, "quiet_count")?;
            }
        }
        if let Some(confidence) = row.confidence {
            self.confidence_sum += confidence;
            self.confidence_count = increment_count(self.confidence_count, "confidence_count")?;
        }
        Ok(())
    }

    fn totals(&self) -> ProviderUsageTotals {
        ProviderUsageTotals {
            total_outcomes: self.total_outcomes,
            candidate_count: self.candidate_count,
            quiet_count: self.quiet_count,
            quiet_rate: self.rate(self.quiet_count),
            candidate_rate: self.rate(self.candidate_count),
        }
    }

    fn average_confidence(&self) -> Option<f64> {
        if self.confidence_count == 0 {
            None
        } else {
            Some(self.confidence_sum / f64::from(self.confidence_count))
        }
    }

    fn rate(&self, count: u32) -> f64 {
        if self.total_outcomes == 0 {
            0.0
        } else {
            f64::from(count) / f64::from(self.total_outcomes)
        }
    }
}

#[derive(Debug, Clone)]
struct ProviderAccumulator {
    provider_id: String,
    counts: CountAccumulator,
    models: BTreeMap<String, ModelAccumulator>,
}

impl ProviderAccumulator {
    fn new(provider_id: String) -> Self {
        Self {
            provider_id,
            counts: CountAccumulator::default(),
            models: BTreeMap::new(),
        }
    }

    fn add(&mut self, row: &ProviderUsageRow) -> Result<(), StorageError> {
        self.counts.add(row)?;
        self.models
            .entry(row.model_id.clone())
            .or_insert_with(|| ModelAccumulator::new(row.model_id.clone()))
            .add(row)
    }

    fn into_provider(self) -> ProviderUsageProvider {
        let mut models = self
            .models
            .into_values()
            .map(ModelAccumulator::into_model)
            .collect::<Vec<_>>();
        sort_models(&mut models);
        ProviderUsageProvider {
            provider_id: self.provider_id,
            total_outcomes: self.counts.total_outcomes,
            candidate_count: self.counts.candidate_count,
            quiet_count: self.counts.quiet_count,
            quiet_rate: self.counts.rate(self.counts.quiet_count),
            candidate_rate: self.counts.rate(self.counts.candidate_count),
            average_confidence: self.counts.average_confidence(),
            models,
        }
    }
}

#[derive(Debug, Clone)]
struct ModelAccumulator {
    model_id: String,
    counts: CountAccumulator,
    prompt_versions: BTreeMap<String, PromptAccumulator>,
}

impl ModelAccumulator {
    fn new(model_id: String) -> Self {
        Self {
            model_id,
            counts: CountAccumulator::default(),
            prompt_versions: BTreeMap::new(),
        }
    }

    fn add(&mut self, row: &ProviderUsageRow) -> Result<(), StorageError> {
        self.counts.add(row)?;
        self.prompt_versions
            .entry(row.prompt_version.clone())
            .or_insert_with(|| PromptAccumulator::new(row.prompt_version.clone()))
            .add(row)
    }

    fn into_model(self) -> ProviderUsageModel {
        let mut prompt_versions = self
            .prompt_versions
            .into_values()
            .map(PromptAccumulator::into_prompt_version)
            .collect::<Vec<_>>();
        sort_prompt_versions(&mut prompt_versions);
        ProviderUsageModel {
            model_id: self.model_id,
            total_outcomes: self.counts.total_outcomes,
            candidate_count: self.counts.candidate_count,
            quiet_count: self.counts.quiet_count,
            quiet_rate: self.counts.rate(self.counts.quiet_count),
            candidate_rate: self.counts.rate(self.counts.candidate_count),
            average_confidence: self.counts.average_confidence(),
            prompt_versions,
        }
    }
}

#[derive(Debug, Clone)]
struct PromptAccumulator {
    prompt_version: String,
    counts: CountAccumulator,
}

impl PromptAccumulator {
    fn new(prompt_version: String) -> Self {
        Self {
            prompt_version,
            counts: CountAccumulator::default(),
        }
    }

    fn add(&mut self, row: &ProviderUsageRow) -> Result<(), StorageError> {
        self.counts.add(row)
    }

    fn into_prompt_version(self) -> ProviderUsagePromptVersion {
        ProviderUsagePromptVersion {
            prompt_version: self.prompt_version,
            total_outcomes: self.counts.total_outcomes,
            candidate_count: self.counts.candidate_count,
            quiet_count: self.counts.quiet_count,
            quiet_rate: self.counts.rate(self.counts.quiet_count),
            candidate_rate: self.counts.rate(self.counts.candidate_count),
            average_confidence: self.counts.average_confidence(),
        }
    }
}

pub(super) fn build_report(
    generated_at_unix_seconds: i64,
    window: ProviderUsageWindow,
    rows: &[ProviderUsageRow],
) -> Result<ProviderUsageReport, StorageError> {
    let mut totals = CountAccumulator::default();
    let mut providers = BTreeMap::<String, ProviderAccumulator>::new();
    for row in rows {
        totals.add(row)?;
        providers
            .entry(row.provider_id.clone())
            .or_insert_with(|| ProviderAccumulator::new(row.provider_id.clone()))
            .add(row)?;
    }
    let mut providers = providers
        .into_values()
        .map(ProviderAccumulator::into_provider)
        .collect::<Vec<_>>();
    sort_providers(&mut providers);
    let recent_outcomes = recent_outcomes(rows);
    Ok(ProviderUsageReport {
        generated_at_unix_seconds,
        window,
        totals: totals.totals(),
        providers,
        recent_outcomes,
    })
}

fn recent_outcomes(rows: &[ProviderUsageRow]) -> Vec<ProviderUsageRecentOutcome> {
    let mut recent_rows = rows.iter().collect::<Vec<_>>();
    recent_rows.sort_by(|left, right| {
        right
            .created_at_unix_seconds
            .cmp(&left.created_at_unix_seconds)
            .then_with(|| right.id.cmp(&left.id))
    });
    recent_rows
        .into_iter()
        .take(PROVIDER_USAGE_RECENT_OUTCOME_LIMIT)
        .map(|row| ProviderUsageRecentOutcome {
            provider_id: row.provider_id.clone(),
            model_id: row.model_id.clone(),
            prompt_version: row.prompt_version.clone(),
            outcome_kind: row.outcome_kind.as_str().to_owned(),
            route_label: row.route_label.clone(),
            confidence: row.confidence,
            created_at_unix_seconds: row.created_at_unix_seconds,
        })
        .collect()
}

fn sort_providers(providers: &mut [ProviderUsageProvider]) {
    providers.sort_by(|left, right| {
        descending_count(left.total_outcomes, right.total_outcomes)
            .then_with(|| left.provider_id.cmp(&right.provider_id))
    });
}

fn sort_models(models: &mut [ProviderUsageModel]) {
    models.sort_by(|left, right| {
        descending_count(left.total_outcomes, right.total_outcomes)
            .then_with(|| left.model_id.cmp(&right.model_id))
    });
}

fn sort_prompt_versions(prompt_versions: &mut [ProviderUsagePromptVersion]) {
    prompt_versions.sort_by(|left, right| {
        descending_count(left.total_outcomes, right.total_outcomes)
            .then_with(|| left.prompt_version.cmp(&right.prompt_version))
    });
}

fn descending_count(left: u32, right: u32) -> Ordering {
    right.cmp(&left)
}

fn increment_count(count: u32, field: &'static str) -> Result<u32, StorageError> {
    count
        .checked_add(1)
        .ok_or_else(|| StorageError::InvalidInput {
            field,
            reason: "provider usage count exceeded u32 range".to_owned(),
        })
}
