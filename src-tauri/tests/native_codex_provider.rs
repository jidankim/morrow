#[path = "support/codex_provider.rs"]
pub mod codex_provider;
#[path = "support/provider.rs"]
pub mod provider;

mod support {
    pub use crate::codex_provider;
    pub use crate::provider;
}

#[path = "native_codex_provider/list_intake_extractor.rs"]
mod list_intake_extractor;
#[path = "native_codex_provider/list_reminder_contract.rs"]
mod list_reminder_contract;
#[path = "native_codex_provider/scheduling_intent.rs"]
mod scheduling_intent;

use morrow_detection::{DetectionOutcome, DetectionPipeline};
use morrow_lib::native_bridge::{CodexCommandOutput, CodexProvider, CodexProviderError};

use support::codex_provider::{FakeCodexRunner, FakeOutcome};
use support::provider::{candidate_json, config, message};

#[test]
fn native_codex_provider_extracts_valid_candidate_json() -> Result<(), String> {
    // Given
    let runner = FakeCodexRunner::new(vec![FakeOutcome::WriteOutput(candidate_json().to_owned())]);
    let provider = CodexProvider::new(&runner);
    let pipeline = DetectionPipeline::new(&provider);
    let evidence = vec![message(
        "chat-a",
        "msg-ambiguous-1",
        "Can we meet Friday afternoon?",
    )?];

    // When
    let report = pipeline.detect(&evidence, &config()?);

    // Then
    let candidate = match report.outcomes.as_slice() {
        [DetectionOutcome::Candidate(candidate), ..] => candidate,
        [DetectionOutcome::QuietLog(quiet), ..] => {
            return Err(format!("expected candidate, got {}", quiet.reason));
        }
        [DetectionOutcome::CachedProviderRoute { .. }, ..] => {
            return Err("expected candidate, got cached provider route".to_owned());
        }
        [] => return Err("missing detection outcome".to_owned()),
    };
    assert_eq!(candidate.title, "Provider meeting");
    assert_eq!(runner.observation_count(), 1);
    Ok(())
}

#[test]
fn native_codex_provider_rejects_invalid_provider_outputs() -> Result<(), String> {
    // Given
    let cases = [
        (
            "non_json",
            FakeOutcome::WriteOutput("I cannot do that.".to_owned()),
        ),
        (
            "refusal",
            FakeOutcome::WriteOutput("{\"refusal\":\"no\"}".to_owned()),
        ),
        (
            "unknown_fields",
            FakeOutcome::WriteOutput(format!(
                "{{{},\"extra\":\"nope\"}}",
                &candidate_json()[1..candidate_json().len() - 1]
            )),
        ),
        ("missing_cli", FakeOutcome::MissingCli),
        ("timeout", FakeOutcome::Timeout),
    ];
    for (name, outcome) in cases {
        let runner = FakeCodexRunner::new(vec![outcome]);
        let provider = CodexProvider::new(&runner);

        // When
        let error = provider
            .extract_response(
                &[message(
                    "chat-a",
                    "msg-ambiguous-1",
                    "Maybe meet tomorrow?",
                )?],
                "Asia/Seoul",
            )
            .err()
            .ok_or_else(|| format!("{name}: extraction unexpectedly succeeded"))?;

        // Then
        let display = error.to_string();
        assert!(
            display.contains("provider"),
            "{name}: expected sanitized provider error, got {display}"
        );
    }
    Ok(())
}

fn candidate_json_with_normalized_time(normalized_time: &str) -> String {
    candidate_json().replace("2026-06-26T15:00:00[Asia/Seoul]", normalized_time)
}

#[test]
fn native_codex_provider_rejects_malformed_normalized_time() -> Result<(), String> {
    for normalized_time in [
        "2026-06-26T15:00:00",
        "2026-06-26T15:00:00+09:00",
        "2026-6-26T15:00:00Z",
        "2026-06-26T15:00:00raw-suffix",
        "2026-06-26T15:00:00[private_clinic_visit]",
        "tomorrow at 3pm",
    ] {
        // Given
        let runner = FakeCodexRunner::new(vec![FakeOutcome::WriteOutput(
            candidate_json_with_normalized_time(normalized_time),
        )]);
        let provider = CodexProvider::new(&runner);

        // When
        let error = provider
            .extract_response(
                &[message(
                    "chat-a",
                    "msg-ambiguous-1",
                    "Maybe meet tomorrow?",
                )?],
                "Asia/Seoul",
            )
            .err()
            .ok_or_else(|| format!("{normalized_time}: extraction unexpectedly succeeded"))?;

        // Then
        assert!(error
            .to_string()
            .contains("candidate normalized_time was invalid"));
    }
    Ok(())
}

#[test]
fn native_codex_provider_redacts_command_errors() -> Result<(), String> {
    // Given
    let runner = FakeCodexRunner::new(vec![FakeOutcome::Completed(CodexCommandOutput::new(
        Some(1),
        "stdout has sk-secret-stdout",
        "stderr has codex_access_token=secret-stderr and raw message text",
    ))]);
    let provider = CodexProvider::new(&runner);

    // When
    let error = provider.extract_response(
        &[message(
            "chat-secret",
            "msg-ambiguous-1",
            "raw prompt evidence with sk-secret-prompt",
        )?],
        "Asia/Seoul",
    );

    // Then
    let provider_error = match error {
        Err(error) => error,
        Ok(response) => return Err(format!("unexpected provider response: {response:?}")),
    };
    let display = provider_error.to_string();
    for forbidden in [
        "sk-secret-stdout",
        "secret-stderr",
        "raw message text",
        "sk-secret-prompt",
        "chat-secret",
    ] {
        assert!(
            !display.contains(forbidden),
            "provider error leaked {forbidden}: {display}"
        );
    }
    assert_eq!(provider_error, CodexProviderError::CommandFailed);
    assert_eq!(display, "codex provider command failed");
    Ok(())
}
