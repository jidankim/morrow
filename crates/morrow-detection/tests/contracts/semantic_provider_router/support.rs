use std::cell::RefCell;

use morrow_detection::{
    AiProvider, DetectionPipeline, ProviderError, ProviderRequest, ProviderResponse,
};
use morrow_storage::CandidateKind;

use super::TestResult;
use crate::support::{config, message, only_candidate, only_quiet, FakeProvider};

pub(super) fn assert_provider_case(
    message_guid: &str,
    excerpt: &str,
    kind: CandidateKind,
    title: &str,
    normalized_time: &str,
) -> TestResult {
    let response = provider_response(kind, title, normalized_time, message_guid);
    let provider = FakeProvider::from_owned(Some(response));
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message("chat-1", message_guid, excerpt, false)?];
    let config = config(550)?;

    let report = pipeline.detect(&messages, &config);

    assert_eq!(
        provider.calls(),
        1,
        "{message_guid} should route to provider"
    );
    let candidate = only_candidate(&report.outcomes)?;
    assert_eq!(candidate.kind, kind);
    assert_eq!(candidate.title, title);
    assert_eq!(candidate.normalized_time, normalized_time);
    assert_eq!(candidate.anchor_message_guid, message_guid);
    assert_eq!(candidate.evidence_excerpt, excerpt.replace('\n', " "));
    Ok(())
}

pub(super) fn assert_local_case(
    message_guid: &str,
    excerpt: &str,
    normalized_time: &str,
) -> TestResult {
    let provider = FakeProvider::new(None);
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message("chat-1", message_guid, excerpt, false)?];
    let config = config(550)?;

    let report = pipeline.detect(&messages, &config);

    assert_eq!(provider.calls(), 0, "{message_guid} should stay local");
    let candidate = only_candidate(&report.outcomes)?;
    assert_eq!(candidate.kind, CandidateKind::CalendarEvent);
    assert_eq!(candidate.title, excerpt);
    assert_eq!(candidate.normalized_time, normalized_time);
    assert_eq!(candidate.anchor_message_guid, message_guid);
    Ok(())
}

pub(super) fn assert_quiet_without_provider(message_guid: &str, excerpt: &str) -> TestResult {
    let provider = FakeProvider::new(None);
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message("chat-1", message_guid, excerpt, false)?];
    let config = config(550)?;

    let report = pipeline.detect(&messages, &config);

    assert_eq!(
        provider.calls(),
        0,
        "{message_guid} should not call provider"
    );
    assert_eq!(report.candidates().count(), 0);
    let quiet = only_quiet(&report.outcomes)?;
    assert!(quiet.reason.starts_with("deterministic_stop:"));
    Ok(())
}

pub(super) fn assert_provider_rejection(
    message_guid: &str,
    excerpt: &str,
    response: &str,
    reason: &str,
) -> TestResult {
    let provider = FakeProvider::new(Some(response));
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message("chat-1", message_guid, excerpt, false)?];
    let config = config(550)?;

    let report = pipeline.detect(&messages, &config);

    assert_eq!(
        provider.calls(),
        1,
        "{message_guid} should route to provider"
    );
    assert_eq!(report.candidates().count(), 0);
    let quiet = only_quiet(&report.outcomes)?;
    assert_eq!(quiet.reason, reason);
    Ok(())
}

pub(super) struct InspectingProvider {
    response: String,
    request_guids: RefCell<Vec<Vec<String>>>,
}

impl InspectingProvider {
    pub(super) fn new(response: &str) -> Self {
        Self {
            response: response.to_owned(),
            request_guids: RefCell::new(Vec::new()),
        }
    }

    pub(super) fn request_guids(&self) -> Vec<Vec<String>> {
        self.request_guids.borrow().clone()
    }
}

impl AiProvider for InspectingProvider {
    fn extract(&self, request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        let guids = request
            .evidence()
            .iter()
            .map(|message| message.message_guid.as_str().to_owned())
            .collect::<Vec<_>>();
        self.request_guids.borrow_mut().push(guids);
        Ok(ProviderResponse::new(&self.response))
    }
}

fn provider_response(
    kind: CandidateKind,
    title: &str,
    normalized_time: &str,
    message_guid: &str,
) -> String {
    format!(
        r#"{{"kind":"{}","title":"{}",
         "confidence_millis":820,
         "normalized_time":"{}",
         "anchor_message_guid":"{}",
         "evidence_message_guids":["{}"]}}"#,
        kind.as_str(),
        title,
        normalized_time,
        message_guid,
        message_guid
    )
}
