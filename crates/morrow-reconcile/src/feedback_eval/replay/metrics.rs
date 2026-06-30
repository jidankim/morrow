use std::collections::BTreeMap;

use morrow_storage::{
    DetectionRouteLabel, EvalCase, EvalResultOutcome, EvalRunStatus, FeedbackLabelValue,
    ProposalOutcomeLabel, SystemOutcomeLabel, FEEDBACK_EVAL_SCHEMA_VERSION,
};

use crate::feedback_eval::{FeedbackEvalReport, ReplayResult, SKIPPED_PROVIDER_NETWORK_DISABLED};

use super::{deterministic_route, visible_candidate};

#[derive(Debug)]
pub(super) struct MetricAccumulator {
    pub(super) cases_evaluated: i64,
    cases_skipped: i64,
    accepted_expected: i64,
    accepted_kept: i64,
    accepted_visible: i64,
    rejection_expected: i64,
    quiet_log_count: i64,
    provider_failure_count: i64,
    pub(super) saw_provider_skip: bool,
    pub(super) saw_failed_validation: bool,
    skip_reasons: BTreeMap<String, i64>,
    confusion_counts: BTreeMap<String, i64>,
}

impl MetricAccumulator {
    pub(super) fn new() -> Self {
        Self {
            cases_evaluated: 0,
            cases_skipped: 0,
            accepted_expected: 0,
            accepted_kept: 0,
            accepted_visible: 0,
            rejection_expected: 0,
            quiet_log_count: 0,
            provider_failure_count: 0,
            saw_provider_skip: false,
            saw_failed_validation: false,
            skip_reasons: BTreeMap::new(),
            confusion_counts: BTreeMap::new(),
        }
    }

    pub(super) fn record(&mut self, case: &EvalCase, replay: &ReplayResult) {
        self.record_expected(case);
        match replay.outcome {
            EvalResultOutcome::Skipped => self.record_skip(replay),
            EvalResultOutcome::Match | EvalResultOutcome::Mismatch => {
                self.cases_evaluated += 1;
                if replay.outcome == EvalResultOutcome::Match
                    && matches!(
                        case.label.label_value,
                        FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::Accepted)
                    )
                {
                    self.accepted_kept += 1;
                }
            }
        }
        self.record_confusion(case, replay);
    }

    pub(super) fn increment_skip(&mut self, reason: &str) {
        *self.skip_reasons.entry(reason.to_owned()).or_insert(0) += 1;
    }

    pub(super) fn accepted_visible_ratio_millis(&self) -> i64 {
        rate_millis(self.accepted_visible, self.accepted_expected)
    }

    pub(super) fn report(&self, status: EvalRunStatus, eval_run_id: i64) -> FeedbackEvalReport {
        FeedbackEvalReport {
            schema_version: FEEDBACK_EVAL_SCHEMA_VERSION,
            eval_run_id,
            status: status.as_str().to_owned(),
            cases_evaluated: self.cases_evaluated,
            cases_skipped: self.cases_skipped,
            skip_reasons: self.skip_reasons.clone(),
            approval_kept_rate_millis: rate_millis(self.accepted_kept, self.accepted_expected),
            observed_rejection_rate_millis: rate_millis(
                self.rejection_expected,
                self.cases_evaluated,
            ),
            quiet_log_count: self.quiet_log_count,
            provider_failure_count: self.provider_failure_count,
            accepted_visible_ratio_millis: self.accepted_visible_ratio_millis(),
            confusion_counts: self.confusion_counts.clone(),
        }
    }

    fn record_skip(&mut self, replay: &ReplayResult) {
        self.cases_skipped += 1;
        if let Some(reason) = replay.skip_reason.as_deref() {
            self.increment_skip(reason);
            if reason == SKIPPED_PROVIDER_NETWORK_DISABLED {
                self.saw_provider_skip = true;
            }
        }
    }

    fn record_expected(&mut self, case: &EvalCase) {
        match case.label.label_value {
            FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::Accepted) => {
                self.accepted_expected += 1;
                if visible_candidate(case) {
                    self.accepted_visible += 1;
                }
            }
            FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::RejectedObserved) => {
                self.rejection_expected += 1;
            }
            FeedbackLabelValue::SystemOutcome(SystemOutcomeLabel::FailedProvider) => {
                self.provider_failure_count += 1;
            }
            FeedbackLabelValue::SystemOutcome(SystemOutcomeLabel::FailedValidation) => {
                self.saw_failed_validation = true;
            }
            FeedbackLabelValue::DetectionRoute(_)
            | FeedbackLabelValue::ProposalOutcome(
                ProposalOutcomeLabel::PendingEdited | ProposalOutcomeLabel::Unknown,
            )
            | FeedbackLabelValue::SystemOutcome(
                SystemOutcomeLabel::Ok | SystemOutcomeLabel::FailedExternalCreation,
            )
            | FeedbackLabelValue::FieldQuality(_) => {}
        }
        if matches!(
            case.snapshot.meta.subject_type,
            morrow_storage::FeedbackSubjectType::QuietLog
        ) || matches!(
            deterministic_route(case),
            Some(DetectionRouteLabel::QuietStop)
        ) {
            self.quiet_log_count += 1;
        }
    }

    fn record_confusion(&mut self, case: &EvalCase, replay: &ReplayResult) {
        let actual = replay
            .actual
            .map_or_else(|| "skipped".to_owned(), |value| value.as_str().to_owned());
        let key = format!("{}->{actual}", case.label.label_value.as_str());
        *self.confusion_counts.entry(key).or_insert(0) += 1;
    }
}

fn rate_millis(numerator: i64, denominator: i64) -> i64 {
    if denominator == 0 {
        1_000
    } else {
        numerator * 1_000 / denominator
    }
}
