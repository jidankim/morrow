mod error;
mod request;
mod runner;
mod workspace;

use std::fs;

use morrow_detection::{AiProvider, ProviderError, ProviderRequest, ProviderResponse};
use morrow_messages::MessageEvidence;

use super::provider_contract;
pub use error::CodexProviderError;
pub use request::CodexExecRequest;
pub use runner::{CodexCommandOutput, CodexExecRun, CodexExecRunner, ProcessCodexExecRunner};
use workspace::CodexTempWorkspace;

pub struct CodexProvider<R> {
    runner: R,
}

impl<R> CodexProvider<R> {
    pub const fn new(runner: R) -> Self {
        Self { runner }
    }
}

impl<R: CodexExecRunner> CodexProvider<R> {
    pub fn extract_response(
        &self,
        evidence: &[MessageEvidence],
    ) -> Result<ProviderResponse, CodexProviderError> {
        let prompt = provider_prompt(evidence)?;
        let schema_text = serde_json::to_string(&provider_contract::candidate_schema())
            .map_err(|_| CodexProviderError::WorkspaceUnavailable)?;
        let workspace = CodexTempWorkspace::create()?;
        fs::write(workspace.schema_path(), schema_text)
            .map_err(|_| CodexProviderError::WorkspaceUnavailable)?;

        let request = CodexExecRequest::for_workspace(&workspace, prompt);
        match self.runner.run_exec(&request) {
            CodexExecRun::Completed(output) if output.exit_code() == Some(0) => {
                let candidate = fs::read_to_string(workspace.output_path())
                    .map_err(|_| CodexProviderError::OutputUnavailable)?;
                provider_contract::validate_candidate_json(&candidate, evidence)?;
                Ok(ProviderResponse::new(&candidate))
            }
            CodexExecRun::Completed(_) => Err(CodexProviderError::CommandFailed),
            CodexExecRun::MissingCli => Err(CodexProviderError::MissingCli),
            CodexExecRun::TimedOut => Err(CodexProviderError::Timeout),
            CodexExecRun::FailedToStart => Err(CodexProviderError::CommandFailed),
        }
    }
}

impl<R: CodexExecRunner> AiProvider for CodexProvider<R> {
    fn extract(&self, request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        self.extract_response(request.evidence())
            .map_err(|error| ProviderError::Unavailable {
                reason: error.to_string(),
            })
    }
}

fn provider_prompt(evidence: &[MessageEvidence]) -> Result<String, CodexProviderError> {
    let evidence_text = provider_contract::evidence_payload_text(evidence)?;
    Ok(format!(
        "Return only one JSON object matching the supplied schema. Use only this redacted evidence payload. \
For an explicit request to create, add, or schedule a calendar event with a date and time, set confidence_millis between 850 and 1000. \
Use confidence_millis below 550 only when the evidence lacks calendar/reminder intent or lacks an inferable time. \
Normalize month-name dates and AM/PM times to ISO-like local time.\n{evidence_text}"
    ))
}

#[cfg(test)]
mod tests {
    use morrow_messages::{ChatGuid, MessageEvidence, MessageGuid, MessageTimestamp};

    use super::provider_prompt;

    #[test]
    fn provider_prompt_calibrates_explicit_calendar_event_confidence() -> Result<(), String> {
        // Given
        let evidence = [MessageEvidence {
            chat_guid: ChatGuid::parse("chat-a").map_err(|error| error.to_string())?,
            message_guid: MessageGuid::parse("msg-a").map_err(|error| error.to_string())?,
            timestamp: MessageTimestamp::new(1_782_705_142).map_err(|error| error.to_string())?,
            participant_count: 1,
            tapback_signal: false,
            excerpt: "Morrow QA: create a calendar event for July 2, 2026 at 3:30 PM.".to_owned(),
            evidence_pointer: "messages://chat-a/msg-a".to_owned(),
        }];

        // When
        let prompt = provider_prompt(&evidence).map_err(|error| error.to_string())?;

        // Then
        assert!(prompt.contains("confidence_millis between 850 and 1000"));
        assert!(prompt.contains("below 550 only"));
        assert!(prompt.contains("Normalize month-name dates and AM/PM times"));
        assert!(prompt.contains("Morrow QA"));
        assert!(!prompt.contains("messages://chat-a/msg-a"));
        Ok(())
    }
}
