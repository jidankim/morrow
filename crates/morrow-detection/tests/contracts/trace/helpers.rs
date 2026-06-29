use std::cell::{Cell, RefCell};
use std::error::Error;

use morrow_detection::{AiProvider, ProviderError, ProviderRequest, ProviderResponse};
use morrow_diagnostics::{
    validate_trace_record_privacy, TraceRecord, TraceRecorder, TraceRecorderError,
};

use super::expectations::ExpectedTrace;

#[derive(Debug, Default)]
pub(super) struct CollectingRecorder {
    records: RefCell<Vec<TraceRecord>>,
    privacy_failures: RefCell<Vec<String>>,
}

impl CollectingRecorder {
    pub(super) fn records(&self) -> Result<Vec<TraceRecord>, Box<dyn Error>> {
        let failures = self.privacy_failures.borrow();
        if !failures.is_empty() {
            return Err(format!("trace privacy validation failed: {}", failures.join("; ")).into());
        }
        let records = self.records.borrow().clone();
        for record in &records {
            validate_trace_record_privacy(record)?;
        }
        Ok(records)
    }
}

impl TraceRecorder for CollectingRecorder {
    fn record(&self, record: &TraceRecord) -> Result<(), TraceRecorderError> {
        if let Err(error) = validate_trace_record_privacy(record) {
            self.privacy_failures.borrow_mut().push(error.to_string());
        }
        self.records.borrow_mut().push(record.clone());
        Ok(())
    }
}

#[derive(Debug, Default)]
pub(super) struct UnavailableProvider {
    calls: Cell<usize>,
}

impl UnavailableProvider {
    pub(super) const fn calls(&self) -> usize {
        self.calls.get()
    }
}

impl AiProvider for UnavailableProvider {
    fn extract(&self, _request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        self.calls.set(self.calls.get() + 1);
        Err(ProviderError::Unavailable {
            reason: "network offline".to_owned(),
        })
    }
}

pub(super) fn assert_trace_sequence(records: &[TraceRecord], expected: &[ExpectedTrace]) {
    assert_eq!(records.len(), expected.len());
    for (record, expected) in records.iter().zip(expected) {
        assert_eq!(record.span.operation, expected.operation);
        assert_eq!(record.span.decision, expected.decision);
        assert_eq!(record.span.outcome, expected.outcome);
        assert_eq!(record.span.reason_code.as_deref(), expected.reason_code);
    }
}

pub(super) fn assert_provider_metadata_prefix(records: &[TraceRecord], expected_count: usize) {
    assert_eq!(records.iter().take(expected_count).count(), expected_count);
    for record in records.iter().take(expected_count) {
        assert_eq!(record.span.provider_id.as_deref(), Some("fake-provider"));
        assert_eq!(record.span.model_id.as_deref(), Some("offline-contract"));
        assert_eq!(record.span.template_version.as_deref(), Some("prompt-v1"));
    }
}

pub(super) fn assert_title_hash_only(records: &[TraceRecord], expected_count: usize) {
    assert_eq!(
        records
            .iter()
            .filter(|record| record.span.title_hash.is_some())
            .count(),
        expected_count
    );
    assert_eq!(
        records
            .iter()
            .filter(|record| record.span.title_status.as_deref() == Some("hashed"))
            .count(),
        expected_count
    );
    assert!(records.iter().all(|record| matches!(
        record.span.title_status.as_deref(),
        Some("hashed" | "absent")
    )));
}

pub(super) fn assert_trace_excludes(
    records: &[TraceRecord],
    forbidden: &[&str],
) -> Result<(), Box<dyn Error>> {
    let serialized = serde_json::to_string(records)?;
    for needle in forbidden {
        assert!(!serialized.contains(needle), "trace leaked {needle}");
    }
    Ok(())
}
