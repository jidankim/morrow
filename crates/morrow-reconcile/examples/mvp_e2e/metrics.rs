#![allow(clippy::redundant_pub_crate)]

use std::path::Path;

pub(crate) struct MetricsInput {
    pub(crate) visible_proposals: u64,
    pub(crate) approvals: u64,
    pub(crate) deletions: u64,
    pub(crate) false_positives: u64,
    pub(crate) latency_seconds_total: u64,
    pub(crate) quiet_logs: usize,
    pub(crate) user_days: u64,
}

pub(crate) struct MetricsReport {
    pub(crate) approval_rate_millis: u64,
    pub(crate) deletion_rate_millis: u64,
    pub(crate) false_positives: u64,
    pub(crate) average_latency_seconds: u64,
    pub(crate) proposals_per_user_day: u64,
    pub(crate) quiet_log_candidates: usize,
}

pub(crate) fn calculate(input: &MetricsInput) -> Result<MetricsReport, Box<dyn std::error::Error>> {
    if input.visible_proposals == 0 || input.user_days == 0 {
        return Err("metrics require visible proposals and user-days".into());
    }
    Ok(MetricsReport {
        approval_rate_millis: rate_millis(input.approvals, input.visible_proposals),
        deletion_rate_millis: rate_millis(input.deletions, input.visible_proposals),
        false_positives: input.false_positives,
        average_latency_seconds: input.latency_seconds_total / input.visible_proposals,
        proposals_per_user_day: input.visible_proposals / input.user_days,
        quiet_log_candidates: input.quiet_logs,
    })
}

impl MetricsReport {
    pub(crate) fn write_report(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::write(
            path,
            format!(
                "approval_rate_millis={}\ndeletion_rate_millis={}\nfalse_positives={}\n\
                 average_latency_seconds={}\nproposals_per_user_day={}\n\
                 quiet_log_candidates={}\n",
                self.approval_rate_millis,
                self.deletion_rate_millis,
                self.false_positives,
                self.average_latency_seconds,
                self.proposals_per_user_day,
                self.quiet_log_candidates
            ),
        )?;
        Ok(())
    }
}

const fn rate_millis(numerator: u64, denominator: u64) -> u64 {
    numerator * 1_000 / denominator
}
