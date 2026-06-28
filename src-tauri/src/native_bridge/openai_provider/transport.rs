use std::time::Duration;

use serde_json::Value;

use super::{OpenAiHttpRequest, OpenAiHttpResponse, OpenAiProviderError, OpenAiTransport};

#[derive(Debug, Clone, Default)]
pub struct ReqwestOpenAiTransport {
    client: reqwest::Client,
}

impl OpenAiTransport for ReqwestOpenAiTransport {
    fn post(&self, request: OpenAiHttpRequest) -> Result<OpenAiHttpResponse, OpenAiProviderError> {
        tauri::async_runtime::block_on(async {
            let response = self
                .client
                .post(&request.url)
                .bearer_auth(request.token())
                .timeout(Duration::from_millis(request.timeout_ms))
                .json(&request.body)
                .send()
                .await
                .map_err(map_reqwest_error)?;
            let status_code = response.status().as_u16();
            let text = response.text().await.map_err(map_reqwest_error)?;
            let body = serde_json::from_str(&text).unwrap_or(Value::Null);
            Ok(OpenAiHttpResponse { status_code, body })
        })
    }
}

fn map_reqwest_error(error: reqwest::Error) -> OpenAiProviderError {
    if error.is_timeout() {
        OpenAiProviderError::Timeout
    } else {
        OpenAiProviderError::NetworkUnavailable
    }
}
