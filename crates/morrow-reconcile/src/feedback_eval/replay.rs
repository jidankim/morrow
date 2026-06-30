use std::path::Path;

use morrow_storage::{
    DetectionRouteLabel, EvalCase, EvalResult, EvalResultOutcome, EvalRunStatus,
    FeedbackLabelValue, ProposalOutcomeLabel,
};

use crate::feedback_eval::{
    eval_run_from_report, ComputedEval, ReplayResult, RunStamp, EMPTY_DATASET,
    SKIPPED_PROVIDER_NETWORK_DISABLED,
};

mod metrics;

pub(crate) fn compute_report(
    cases: &[EvalCase],
    run_stamp: &RunStamp,
    report_path: &Path,
) -> ComputedEval {
    let mut metrics = metrics::MetricAccumulator::new();
    let mut results = Vec::with_capacity(cases.len());
    let run_key = run_stamp.key(cases.len());
    for case in cases {
        let replay = replay_case(case);
        metrics.record(case, &replay);
        results.push(eval_result(case, &replay, &run_key, run_stamp.seconds));
    }
    if cases.is_empty() {
        metrics.increment_skip(EMPTY_DATASET);
    }
    let status = derive_status(&metrics);
    let report = metrics.report(status, 0);
    let run = eval_run_from_report(run_key, status, run_stamp, report_path, &report);
    ComputedEval {
        report,
        run,
        results,
    }
}

fn replay_case(case: &EvalCase) -> ReplayResult {
    if provider_route(case) {
        return ReplayResult {
            actual: None,
            outcome: EvalResultOutcome::Skipped,
            skip_reason: Some(SKIPPED_PROVIDER_NETWORK_DISABLED.to_owned()),
        };
    }
    let actual = actual_label(case);
    let outcome = match actual {
        Some(value) if value == case.label.label_value => EvalResultOutcome::Match,
        Some(_) | None => EvalResultOutcome::Mismatch,
    };
    ReplayResult {
        actual,
        outcome,
        skip_reason: None,
    }
}

fn actual_label(case: &EvalCase) -> Option<FeedbackLabelValue> {
    match case.label.label_value {
        FeedbackLabelValue::DetectionRoute(_) => {
            deterministic_route(case).map(FeedbackLabelValue::DetectionRoute)
        }
        FeedbackLabelValue::ProposalOutcome(_) if visible_candidate(case) => Some(
            FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::Accepted),
        ),
        FeedbackLabelValue::ProposalOutcome(_) => Some(FeedbackLabelValue::ProposalOutcome(
            ProposalOutcomeLabel::Unknown,
        )),
        FeedbackLabelValue::SystemOutcome(value) => Some(FeedbackLabelValue::SystemOutcome(value)),
        FeedbackLabelValue::FieldQuality(value) => Some(FeedbackLabelValue::FieldQuality(value)),
    }
}

fn provider_route(case: &EvalCase) -> bool {
    matches!(
        case.snapshot.route.as_deref(),
        Some("provider_candidate" | "provider_rejection")
    ) || matches!(
        case.label.label_value,
        FeedbackLabelValue::DetectionRoute(DetectionRouteLabel::ProviderCandidate)
            | FeedbackLabelValue::DetectionRoute(DetectionRouteLabel::ProviderRejected)
            | FeedbackLabelValue::DetectionRoute(DetectionRouteLabel::ProviderUnavailable)
    )
}

pub(super) fn deterministic_route(case: &EvalCase) -> Option<DetectionRouteLabel> {
    match case.snapshot.route.as_deref() {
        Some("deterministic_candidate") => Some(DetectionRouteLabel::DeterministicCandidate),
        Some("deterministic_stop") => Some(DetectionRouteLabel::QuietStop),
        _ => None,
    }
}

pub(super) fn visible_candidate(case: &EvalCase) -> bool {
    matches!(
        deterministic_route(case),
        Some(DetectionRouteLabel::DeterministicCandidate)
    )
}

fn eval_result(
    case: &EvalCase,
    replay: &ReplayResult,
    run_key: &str,
    created_at: i64,
) -> EvalResult {
    EvalResult {
        id: None,
        result_key: format!(
            "{run_key}-result-{}-{}",
            case.snapshot.snapshot_key, case.label.label_key
        ),
        eval_run_id: 0,
        snapshot_key: case.snapshot.snapshot_key.clone(),
        label_key: case.label.label_key.clone(),
        expected_label_type: case.label.label_value.label_type(),
        expected_label_value: case.label.label_value.as_str().to_owned(),
        actual_label_type: replay.actual.map(FeedbackLabelValue::label_type),
        actual_label_value: replay.actual.map(|value| value.as_str().to_owned()),
        outcome: replay.outcome,
        skip_reason: replay.skip_reason.clone(),
        created_at,
    }
}

fn derive_status(metrics: &metrics::MetricAccumulator) -> EvalRunStatus {
    if metrics.saw_failed_validation {
        EvalRunStatus::Failed
    } else if metrics.cases_evaluated == 0
        || metrics.saw_provider_skip
        || metrics.accepted_visible_ratio_millis() < 500
    {
        EvalRunStatus::NeedsReview
    } else {
        EvalRunStatus::Passed
    }
}
