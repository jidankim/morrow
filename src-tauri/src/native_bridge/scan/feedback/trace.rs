use std::sync::Mutex;

use morrow_diagnostics::{
    TraceComponent, TraceDecision, TraceOperation, TraceOutcome, TraceRecord, TraceRecorder,
    TraceRecorderError,
};
use morrow_storage::DetectionRouteLabel;

use crate::native_bridge::scan::ScanSelectedChatsError;

pub(in crate::native_bridge::scan) struct FeedbackTraceRecorder<'a, R: TraceRecorder + ?Sized> {
    inner: &'a R,
    records: Mutex<Vec<TraceRecord>>,
}

impl<'a, R: TraceRecorder + ?Sized> FeedbackTraceRecorder<'a, R> {
    pub(in crate::native_bridge::scan) const fn new(inner: &'a R) -> Self {
        Self {
            inner,
            records: Mutex::new(Vec::new()),
        }
    }

    pub(in crate::native_bridge::scan) fn trace_groups(
        &self,
    ) -> Result<Vec<TraceGroup>, ScanSelectedChatsError> {
        let records = self
            .records
            .lock()
            .map_err(|_| ScanSelectedChatsError::Detection("trace recorder poisoned".to_owned()))?;
        let mut groups: Vec<TraceGroup> = Vec::new();
        for record in records.iter() {
            match groups
                .iter_mut()
                .find(|group| group.trace_id == record.trace.trace_id)
            {
                Some(group) => group.records.push(record.clone()),
                None => groups.push(TraceGroup {
                    trace_id: record.trace.trace_id.clone(),
                    records: vec![record.clone()],
                }),
            }
        }
        Ok(groups)
    }
}

impl<R: TraceRecorder + ?Sized> TraceRecorder for FeedbackTraceRecorder<'_, R> {
    fn record(&self, record: &TraceRecord) -> Result<(), TraceRecorderError> {
        let inner_result = self.inner.record(record);
        let Ok(mut records) = self.records.lock() else {
            return inner_result;
        };
        records.push(record.clone());
        inner_result
    }
}

#[derive(Debug, Clone)]
pub(in crate::native_bridge::scan) struct TraceGroup {
    trace_id: String,
    records: Vec<TraceRecord>,
}

impl TraceGroup {
    pub(super) fn candidate_record(&self, route: CandidateRoute) -> Option<&TraceRecord> {
        match route {
            CandidateRoute::Provider => self.find_record(
                TraceOperation::ThresholdDecision,
                Some(TraceDecision::ConfidenceAccepted),
            ),
            CandidateRoute::Deterministic => self.find_record(
                TraceOperation::ParserDecision,
                Some(TraceDecision::Candidate),
            ),
        }
        .or_else(|| self.outcome_record(TraceOutcome::CandidateCreated))
    }

    pub(super) fn quiet_record(&self, reason: &str) -> Option<&TraceRecord> {
        self.records
            .iter()
            .find(|record| {
                record.span.reason_code.as_deref() == Some(reason)
                    && record.span.component != TraceComponent::Outcome
            })
            .or_else(|| self.outcome_record(TraceOutcome::QuietLogged))
    }

    fn find_record(
        &self,
        operation: TraceOperation,
        decision: Option<TraceDecision>,
    ) -> Option<&TraceRecord> {
        self.records.iter().find(|record| {
            record.span.operation == operation
                && record.span.decision == decision
                && record.span.outcome != TraceOutcome::Noop
        })
    }

    fn outcome_record(&self, outcome: TraceOutcome) -> Option<&TraceRecord> {
        self.records.iter().find(|record| {
            record.span.operation == TraceOperation::OutcomeMaterialized
                && record.span.outcome == outcome
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum CandidateRoute {
    Deterministic,
    Provider,
}

impl CandidateRoute {
    pub(super) const fn as_snapshot_route(self) -> &'static str {
        match self {
            Self::Deterministic => "deterministic_candidate",
            Self::Provider => "provider_candidate",
        }
    }

    pub(super) const fn as_reason_code(self) -> &'static str {
        match self {
            Self::Deterministic => "parser_candidate",
            Self::Provider => "confidence_meets_threshold",
        }
    }

    pub(super) const fn as_detection_label(self) -> DetectionRouteLabel {
        match self {
            Self::Deterministic => DetectionRouteLabel::DeterministicCandidate,
            Self::Provider => DetectionRouteLabel::ProviderCandidate,
        }
    }
}

pub(super) fn candidate_route(trace_group: Option<&TraceGroup>) -> CandidateRoute {
    let Some(group) = trace_group else {
        return CandidateRoute::Deterministic;
    };
    if group.records.iter().any(|record| {
        record.span.decision == Some(TraceDecision::ProviderRoute)
            || record.span.component == TraceComponent::Provider
            || record.span.component == TraceComponent::Schema
            || record.span.component == TraceComponent::Threshold
    }) {
        CandidateRoute::Provider
    } else {
        CandidateRoute::Deterministic
    }
}
