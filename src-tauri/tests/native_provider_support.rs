use std::cell::RefCell;

use morrow_detection::{ConfidenceThreshold, ProviderIdentity, ReferenceTime, SourceExcerptPolicy};
use morrow_lib::native_bridge::{
    OpenAiHttpRequest, OpenAiHttpResponse, OpenAiProviderError, OpenAiTransport, OPENAI_MODEL,
};
use morrow_messages::{ChatGuid, MessageEvidence, MessageGuid, MessageTimestamp};
use serde_json::{json, Value};

#[derive(Debug)]
pub struct MockTransport {
    pub responses: RefCell<Vec<Result<OpenAiHttpResponse, OpenAiProviderError>>>,
    pub requests: RefCell<Vec<OpenAiHttpRequest>>,
}

impl MockTransport {
    pub fn with_responses(responses: Vec<Result<OpenAiHttpResponse, OpenAiProviderError>>) -> Self {
        Self {
            responses: RefCell::new(responses),
            requests: RefCell::new(Vec::new()),
        }
    }

    pub fn only_request(&self) -> Result<OpenAiHttpRequest, String> {
        let requests = self.requests.borrow();
        assert_eq!(requests.len(), 1);
        requests
            .first()
            .cloned()
            .ok_or_else(|| "missing request".to_owned())
    }
}

impl OpenAiTransport for MockTransport {
    fn post(&self, request: OpenAiHttpRequest) -> Result<OpenAiHttpResponse, OpenAiProviderError> {
        self.requests.borrow_mut().push(request);
        self.responses
            .borrow_mut()
            .pop()
            .ok_or(OpenAiProviderError::NetworkUnavailable)?
    }
}

pub fn candidate_json() -> &'static str {
    "{\"kind\":\"calendar_event\",\"title\":\"Provider meeting\",\"confidence_millis\":800,\
     \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
     \"anchor_message_guid\":\"msg-ambiguous-1\",\
     \"evidence_message_guids\":[\"msg-ambiguous-1\"]}"
}

pub fn completed_response(candidate: &str) -> OpenAiHttpResponse {
    OpenAiHttpResponse {
        status_code: 200,
        body: json!({"status": "completed", "output": [{"type": "message", "role": "assistant", "status": "completed", "content": [{"type": "output_text", "text": candidate}]}]}),
    }
}

pub fn message(
    chat_guid: &str,
    message_guid: &str,
    excerpt: &str,
) -> Result<MessageEvidence, String> {
    Ok(MessageEvidence {
        chat_guid: ChatGuid::parse(chat_guid).map_err(|error| error.to_string())?,
        message_guid: MessageGuid::parse(message_guid).map_err(|error| error.to_string())?,
        timestamp: MessageTimestamp::new(1_782_352_400).map_err(|error| error.to_string())?,
        participant_count: 2,
        tapback_signal: true,
        excerpt: excerpt.to_owned(),
        evidence_pointer: format!("messages://{chat_guid}/{message_guid}"),
    })
}

pub fn config() -> Result<morrow_detection::DetectionConfig, String> {
    Ok(morrow_detection::DetectionConfig {
        reference: ReferenceTime::parse("2026-06-25T09:00:00", "Asia/Seoul")
            .map_err(|error| error.to_string())?,
        threshold: ConfidenceThreshold::new(550).map_err(|error| error.to_string())?,
        provider: ProviderIdentity::new("openai", OPENAI_MODEL, "native-provider-v1")
            .map_err(|error| error.to_string())?,
        source_excerpts: SourceExcerptPolicy::Include,
    })
}

pub fn assert_allowed_evidence_keys(record: &Value) -> Result<(), String> {
    let object = record
        .as_object()
        .ok_or_else(|| "evidence record is not an object".to_owned())?;
    let mut keys = object.keys().map(String::as_str).collect::<Vec<_>>();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "evidence_pointer",
            "excerpt",
            "message_guid",
            "participant_count",
            "tapback_signal",
            "timestamp"
        ]
    );
    Ok(())
}
