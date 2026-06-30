use std::path::Path;

/// Inputs used to calculate the MVP E2E metrics report.
#[derive(Debug)]
pub struct MetricsInput {
    /// Visible proposal count.
    pub visible_proposals: u64,
    /// Approval count.
    pub approvals: u64,
    /// Deletion count.
    pub deletions: u64,
    /// False-positive count.
    pub false_positives: u64,
    /// Total synthetic latency.
    pub latency_seconds_total: u64,
    /// Quiet-log candidate count.
    pub quiet_logs: usize,
    /// Number of user-days represented.
    pub user_days: u64,
}

/// Calculated MVP E2E metrics.
#[derive(Debug)]
pub struct MetricsReport {
    /// Approval rate in thousandths.
    pub approval_rate_millis: u64,
    /// Deletion rate in thousandths.
    pub deletion_rate_millis: u64,
    /// False-positive count.
    pub false_positives: u64,
    /// Average synthetic latency.
    pub average_latency_seconds: u64,
    /// Proposal count per user-day.
    pub proposals_per_user_day: u64,
    /// Quiet-log candidate count.
    pub quiet_log_candidates: usize,
}

#[derive(Debug)]
pub(crate) struct FeedbackEvalMetrics {
    pub(crate) labels_recorded: i64,
    pub(crate) snapshots_recorded: i64,
    pub(crate) eval_results_recorded: i64,
    pub(crate) eval_report: String,
}

/// Calculates the MVP E2E metrics report.
pub fn calculate(input: &MetricsInput) -> Result<MetricsReport, Box<dyn std::error::Error>> {
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
    /// Writes the metrics report as key-value lines.
    pub(crate) fn write_report(
        &self,
        path: &Path,
        feedback: &FeedbackEvalMetrics,
    ) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::write(
            path,
            format!(
                "approval_rate_millis={}\ndeletion_rate_millis={}\nfalse_positives={}\n\
                 average_latency_seconds={}\nproposals_per_user_day={}\n\
                 quiet_log_candidates={}\nlabels_recorded={}\nsnapshots_recorded={}\n\
                 eval_results_recorded={}\neval_report={}\n",
                self.approval_rate_millis,
                self.deletion_rate_millis,
                self.false_positives,
                self.average_latency_seconds,
                self.proposals_per_user_day,
                self.quiet_log_candidates,
                feedback.labels_recorded,
                feedback.snapshots_recorded,
                feedback.eval_results_recorded,
                feedback.eval_report
            ),
        )?;
        Ok(())
    }
}

const fn rate_millis(numerator: u64, denominator: u64) -> u64 {
    numerator * 1_000 / denominator
}
