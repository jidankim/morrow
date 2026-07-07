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
        reference_timezone: &str,
    ) -> Result<ProviderResponse, CodexProviderError> {
        let prompt = provider_prompt(evidence, reference_timezone)?;
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
                let localized = provider_contract::localize_candidate_json(&candidate, evidence)?;
                Ok(ProviderResponse::new(&localized))
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
        self.extract_response(request.evidence(), request.reference_timezone())
            .map_err(|error| ProviderError::Unavailable {
                reason: error.to_string(),
            })
    }
}

fn provider_prompt(
    evidence: &[MessageEvidence],
    reference_timezone: &str,
) -> Result<String, CodexProviderError> {
    let evidence_text = provider_contract::evidence_payload_text(evidence)?;
    let configured_example = match reference_timezone {
        "UTC" => "2026-07-03T15:30:00Z".to_owned(),
        timezone => format!("2026-07-03T15:30:00[{timezone}]"),
    };
    Ok(format!(
        "Return only one JSON object matching the supplied schema. Use only this redacted evidence payload. \
For an explicit request to create, add, or schedule a calendar event with a date and time, set confidence_millis between 850 and 1000. \
For weak calendar wording such as catch up Friday afternoon, coffee Friday afternoon, sync Friday afternoon, or touch base Friday afternoon, set kind to calendar_event and set confidence_millis between 850 and 1000 when the date/time is inferable. \
Semantic calendar wording: natural phrasing such as readouts, check-ins, holds, bookings, visits, or other meeting-like commitments may be calendar_event when selected evidence only grounds the title, context, participants, and time. \
Calendar vs Reminders disambiguation: choose calendar_event for meetings, appointments, calls, visits, holds, bookings, or shared time commitments; choose task_reminder for reminders, todos, follow-ups, deadlines, due dates, or by-date obligations. \
For a task or reminder request with a clear due date, deadline, or by-date, set kind to task_reminder and set confidence_millis between 700 and 850. \
For weak task deadline wording such as follow up by July 25, send by July 25, finish by July 25, complete by July 25, due July 25, or deadline July 25, keep kind as task_reminder. \
If that task/reminder request has a clear deadline date but no clock time, use 23:59:00 in the configured reference timezone. \
For list-reminders-v1 quantity lists such as 2 anchovies, 3 salmon, set kind to task_reminder and include structured items with numeric quantity, normalized name, optional bounded unit, and evidence_ids for each item; the title will be rendered locally as Daily list from validated items. \
Generate title and context from selected evidence only; do not infer private details, names, locations, attendees, or source text that are not present in the redacted selected evidence payload. \
Reject ungrounded or hallucinated evidence: every anchor_evidence_id, evidence_ids entry, and item evidence_ids entry must reference an existing evidence://selected/N id from the payload, and unsupported or invented evidence should drive confidence_millis below 550. \
Use confidence_millis below 550 only when the evidence lacks calendar/reminder intent or lacks an inferable date or time after these calendar/reminder rules. \
normalized_time contract: use exactly YYYY-MM-DDTHH:MM:SS[Area/Location] with the configured reference timezone or YYYY-MM-DDTHH:MM:SSZ for UTC, resolving semantic or relative wording only when grounded by selected evidence and the reference timezone. \
Date and time fields are fixed-width; seconds are mandatory. Bracketed zones must be safe IANA-style zones. \
Valid examples: {configured_example}, 2026-07-03T06:30:00Z. \
Invalid examples: 2026-7-3T15:30Z, 2026-07-03T15:30, 2026-07-03 15:30:00, 2026-07-03T15:30:00+09:00, tomorrow at 3pm, 2026-07-03T15:30:00[Private/Prompt].\n{evidence_text}"
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
        let prompt = provider_prompt(&evidence, "Asia/Seoul").map_err(|error| error.to_string())?;

        // Then
        assert!(prompt.contains("confidence_millis between 850 and 1000"));
        assert!(prompt.contains("below 550 only"));
        assert!(prompt.contains("normalized_time contract"));
        assert!(prompt.contains("YYYY-MM-DDTHH:MM:SS[Area/Location]"));
        assert!(prompt.contains("YYYY-MM-DDTHH:MM:SSZ"));
        assert!(prompt.contains("fixed-width"));
        assert!(prompt.contains("seconds are mandatory"));
        assert!(prompt.contains("safe IANA-style"));
        assert!(prompt.contains("2026-07-03T15:30:00[Asia/Seoul]"));
        assert!(prompt.contains("2026-07-03T06:30:00Z"));
        for invalid in [
            "2026-7-3T15:30Z",
            "2026-07-03T15:30",
            "2026-07-03 15:30:00",
            "2026-07-03T15:30:00+09:00",
            "tomorrow at 3pm",
            "2026-07-03T15:30:00[Private/Prompt]",
        ] {
            assert!(
                prompt.contains(invalid),
                "missing invalid example {invalid}"
            );
        }
        assert!(!prompt.contains("ISO-like local time"));
        assert!(prompt.contains("Morrow QA"));
        assert!(prompt.contains("evidence://selected/0"));
        assert!(!prompt.contains("msg-a"));
        assert!(!prompt.contains("messages://chat-a/msg-a"));
        Ok(())
    }

    #[test]
    fn provider_prompt_calibrates_task_due_date_confidence() -> Result<(), String> {
        // Given
        let evidence = [MessageEvidence {
            chat_guid: ChatGuid::parse("chat-a").map_err(|error| error.to_string())?,
            message_guid: MessageGuid::parse("msg-a").map_err(|error| error.to_string())?,
            timestamp: MessageTimestamp::new(1_782_705_142).map_err(|error| error.to_string())?,
            participant_count: 1,
            tapback_signal: false,
            excerpt: "Finish review of the essay by July 25, 2026.".to_owned(),
            evidence_pointer: "messages://chat-a/msg-a".to_owned(),
        }];

        // When
        let prompt = provider_prompt(&evidence, "Asia/Seoul").map_err(|error| error.to_string())?;

        // Then
        assert!(prompt.contains("task or reminder request"));
        assert!(prompt.contains("due date, deadline, or by-date"));
        assert!(prompt.contains("confidence_millis between 700 and 850"));
        assert!(prompt.contains("23:59:00"));
        assert!(prompt.contains("below 550 only"));
        assert!(prompt.contains("Finish review of the essay"));
        Ok(())
    }
}
