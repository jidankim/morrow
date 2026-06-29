mod payload;
mod response;
mod transport;

use std::fmt;

use morrow_detection::{AiProvider, ProviderError, ProviderRequest, ProviderResponse};
use morrow_messages::MessageEvidence;
use serde_json::Value;

use super::provider_contract::ProviderContractError;
pub use transport::ReqwestOpenAiTransport;

pub const OPENAI_RESPONSES_URL: &str = "https://api.openai.com/v1/responses";
pub const OPENAI_MODEL: &str = "gpt-5.5";
pub const OPENAI_TIMEOUT_MS: u64 = 15_000;

pub struct OpenAiProvider<T> {
    token: String,
    transport: T,
}

impl<T> OpenAiProvider<T> {
    pub fn new(token: impl Into<String>, transport: T) -> Self {
        Self {
            token: token.into(),
            transport,
        }
    }
}

impl<T: OpenAiTransport> OpenAiProvider<T> {
    pub fn extract_response(
        &self,
        evidence: &[MessageEvidence],
    ) -> Result<ProviderResponse, OpenAiProviderError> {
        let body = payload::request_body(evidence)?;
        let request = OpenAiHttpRequest::new(self.token.clone(), body);
        let response = self.transport.post(request)?;
        response::response_candidate(response, evidence)
    }
}

impl<T: OpenAiTransport> AiProvider for OpenAiProvider<T> {
    fn extract(&self, request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        self.extract_response(request.evidence())
            .map_err(|error| ProviderError::Unavailable {
                reason: error.to_string(),
            })
    }
}

impl<T: OpenAiTransport + ?Sized> OpenAiTransport for &T {
    fn post(&self, request: OpenAiHttpRequest) -> Result<OpenAiHttpResponse, OpenAiProviderError> {
        (*self).post(request)
    }
}

#[derive(Clone)]
pub struct OpenAiHttpRequest {
    pub url: String,
    pub timeout_ms: u64,
    pub body: Value,
    token: String,
}

impl OpenAiHttpRequest {
    fn new(token: String, body: Value) -> Self {
        Self {
            url: OPENAI_RESPONSES_URL.to_owned(),
            timeout_ms: OPENAI_TIMEOUT_MS,
            body,
            token,
        }
    }

    pub(super) fn token(&self) -> &str {
        &self.token
    }
}

impl fmt::Debug for OpenAiHttpRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenAiHttpRequest")
            .field("url", &self.url)
            .field("timeout_ms", &self.timeout_ms)
            .field("body", &self.body)
            .field("token", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct OpenAiHttpResponse {
    pub status_code: u16,
    pub body: Value,
}

pub trait OpenAiTransport {
    fn post(&self, request: OpenAiHttpRequest) -> Result<OpenAiHttpResponse, OpenAiProviderError>;
}

#[derive(Debug, Clone)]
pub enum OpenAiProviderError {
    EvidenceTooLarge,
    HttpStatus { status_code: u16 },
    Timeout,
    NetworkUnavailable,
    InvalidResponse { reason: &'static str },
}

impl fmt::Display for OpenAiProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EvidenceTooLarge => write!(formatter, "provider evidence is too large"),
            Self::HttpStatus { status_code } => {
                write!(
                    formatter,
                    "provider response has unsupported status {status_code}"
                )
            }
            Self::Timeout => write!(formatter, "provider request timed out"),
            Self::NetworkUnavailable => write!(formatter, "provider network unavailable"),
            Self::InvalidResponse { reason } => {
                write!(formatter, "provider response rejected: {reason}")
            }
        }
    }
}

impl std::error::Error for OpenAiProviderError {}

impl From<ProviderContractError> for OpenAiProviderError {
    fn from(error: ProviderContractError) -> Self {
        match error {
            ProviderContractError::EvidenceSerialization => Self::InvalidResponse {
                reason: "evidence serialization failed",
            },
            ProviderContractError::EvidenceTooLarge => Self::EvidenceTooLarge,
            ProviderContractError::InvalidCandidate { reason } => Self::InvalidResponse { reason },
        }
    }
}

pub(super) const fn invalid_response(reason: &'static str) -> OpenAiProviderError {
    OpenAiProviderError::InvalidResponse { reason }
}
