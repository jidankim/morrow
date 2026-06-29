use std::cell::RefCell;

use morrow_lib::native_bridge::{
    OpenAiHttpRequest, OpenAiHttpResponse, OpenAiProviderError, OpenAiTransport,
};
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

pub fn completed_response(candidate: &str) -> OpenAiHttpResponse {
    OpenAiHttpResponse {
        status_code: 200,
        body: json!({"status": "completed", "output": [{"type": "message", "role": "assistant", "status": "completed", "content": [{"type": "output_text", "text": candidate}]}]}),
    }
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
