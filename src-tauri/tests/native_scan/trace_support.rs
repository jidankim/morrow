use std::sync::Mutex;

use morrow_detection::{AiProvider, ProviderError, ProviderRequest, ProviderResponse};
use morrow_diagnostics::{TraceRecord, TraceRecorder, TraceRecorderError};
use morrow_lib::native_bridge::ScanSelectedChatsResult;
use serde_json::json;

#[derive(Default)]
pub(super) struct RecordingTraceRecorder(Mutex<Vec<TraceRecord>>);

impl RecordingTraceRecorder {
    pub(super) fn records(&self) -> Result<Vec<TraceRecord>, String> {
        self.0
            .lock()
            .map(|records| records.clone())
            .map_err(|error| error.to_string())
    }
}

impl TraceRecorder for RecordingTraceRecorder {
    fn record(&self, record: &TraceRecord) -> Result<(), TraceRecorderError> {
        let Ok(mut records) = self.0.lock() else {
            return Err(TraceRecorderError::Unavailable);
        };
        records.push(record.clone());
        Ok(())
    }
}

pub(super) struct UnavailableTraceRecorder;

impl TraceRecorder for UnavailableTraceRecorder {
    fn record(&self, _record: &TraceRecord) -> Result<(), TraceRecorderError> {
        Err(TraceRecorderError::Unavailable)
    }
}

pub(super) struct ProviderCandidateStub;

impl AiProvider for ProviderCandidateStub {
    fn extract(&self, request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        let Some(anchor) = request.evidence().first() else {
            return Err(ProviderError::Unavailable {
                reason: "test provider requires one evidence message".to_owned(),
            });
        };
        Ok(ProviderResponse::new(
            &json!({
                "kind": "calendar_event",
                "title": "Provider private title",
                "confidence_millis": 910,
                "normalized_time": "2026-06-27T09:00:00[Asia/Seoul]",
                "anchor_message_guid": anchor.message_guid.as_str(),
                "evidence_message_guids": [anchor.message_guid.as_str()],
            })
            .to_string(),
        ))
    }
}

pub(super) struct ProviderUnavailableStub;

impl AiProvider for ProviderUnavailableStub {
    fn extract(&self, _request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        Err(ProviderError::Unavailable {
            reason: "test provider unavailable".to_owned(),
        })
    }
}

pub(super) fn scan_result_counts(
    result: &ScanSelectedChatsResult,
) -> (usize, usize, usize, usize, usize, usize, usize) {
    (
        result.pending_proposal_count,
        result.created_candidate_count,
        result.quiet_log_count,
        result.cap_visible_count,
        result.cap_deferred_count,
        result.created_external_proposal_count,
        result.failed_external_proposal_count,
    )
}

pub(super) fn assert_trace_records_hide_raw_native_content(
    records: &[TraceRecord],
    forbidden: &[&str],
) -> Result<(), String> {
    let serialized = serde_json::to_string(records).map_err(|error| error.to_string())?;
    for value in forbidden {
        assert!(
            !serialized.contains(value),
            "trace records leaked raw native content {value}: {serialized}"
        );
    }
    Ok(())
}
