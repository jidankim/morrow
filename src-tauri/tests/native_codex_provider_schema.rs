#[path = "support/codex_provider.rs"]
pub mod codex_provider;
#[path = "support/provider.rs"]
pub mod provider;

mod support {
    pub use crate::codex_provider;
    pub use crate::provider;
}

use morrow_detection::{DetectionOutcome, DetectionPipeline, ReferenceTime};
use morrow_lib::native_bridge::CodexProvider;
use regex::Regex;
use serde_json::Value;

use support::codex_provider::{FakeCodexRunner, FakeOutcome};
use support::provider::{candidate_json, config, message};

#[test]
fn native_codex_provider_schema_constrains_normalized_time() -> Result<(), String> {
    // Given
    let runner = FakeCodexRunner::new(vec![FakeOutcome::WriteOutput(candidate_json().to_owned())]);
    let provider = CodexProvider::new(&runner);
    let evidence = [message(
        "chat-a",
        "msg-ambiguous-1",
        "Create a calendar event for July 2, 2026 at 3:30 PM.",
    )?];

    // When
    provider
        .extract_response(&evidence, "Asia/Seoul")
        .map_err(|error| error.to_string())?;

    // Then
    let observation = runner.only_observation()?;
    assert_prompt_includes_normalized_time_contract(
        &observation.prompt_text,
        "2026-07-03T15:30:00[Asia/Seoul]",
    );
    let schema: Value =
        serde_json::from_str(&observation.schema_text).map_err(|error| error.to_string())?;
    let normalized_time_schema = &schema["properties"]["normalized_time"];
    assert_eq!(normalized_time_schema["type"], "string");
    let normalized_time_description = normalized_time_schema["description"]
        .as_str()
        .ok_or_else(|| "missing normalized_time.description".to_owned())?;
    assert!(normalized_time_description.contains("YYYY-MM-DDTHH:MM:SS[Area/Location]"));
    assert!(normalized_time_description.contains("YYYY-MM-DDTHH:MM:SSZ"));
    let normalized_time_pattern = normalized_time_schema["pattern"]
        .as_str()
        .ok_or_else(|| "missing normalized_time.pattern".to_owned())?;
    let normalized_time_regex =
        Regex::new(normalized_time_pattern).map_err(|error| error.to_string())?;
    for accepted in ["2026-06-26T15:00:00Z", "2026-06-26T15:00:00[Asia/Seoul]"] {
        assert!(
            normalized_time_regex.is_match(accepted),
            "pattern rejected {accepted}: {normalized_time_pattern}"
        );
    }
    for rejected in [
        "2026-6-26T15:00:00Z",
        "2026-06-26T15:00Z",
        "2026-06-26 15:00:00",
        "2026-06-26T15:00:00+09:00",
        "tomorrow at 3pm",
    ] {
        assert!(
            !normalized_time_regex.is_match(rejected),
            "pattern accepted {rejected}: {normalized_time_pattern}"
        );
    }
    Ok(())
}

#[test]
fn native_codex_provider_prompt_uses_configured_reference_timezone() -> Result<(), String> {
    // Given
    let runner = FakeCodexRunner::new(vec![FakeOutcome::WriteOutput(candidate_json().to_owned())]);
    let provider = CodexProvider::new(&runner);
    let pipeline = DetectionPipeline::new(&provider);
    let evidence = vec![message(
        "chat-a",
        "msg-ambiguous-1",
        "Can we meet Friday afternoon?",
    )?];
    let mut configured = config()?;
    configured.reference = ReferenceTime::parse("2026-06-25T09:00:00", "America/New_York")
        .map_err(|error| error.to_string())?;

    // When
    let report = pipeline.detect(&evidence, &configured);

    // Then
    match report.outcomes.as_slice() {
        [DetectionOutcome::Candidate(_), ..] => {}
        [DetectionOutcome::QuietLog(quiet), ..] => {
            return Err(format!("expected candidate, got {}", quiet.reason));
        }
        [DetectionOutcome::CachedProviderRoute { .. }, ..] => {
            return Err("expected candidate, got cached provider route".to_owned());
        }
        [] => return Err("missing detection outcome".to_owned()),
    }
    let observation = runner.only_observation()?;
    assert_prompt_includes_normalized_time_contract(
        &observation.prompt_text,
        "2026-07-03T15:30:00[America/New_York]",
    );
    assert!(!observation
        .prompt_text
        .contains("2026-07-03T15:30:00[Asia/Seoul]"));
    Ok(())
}

#[test]
fn native_codex_provider_prompt_renders_utc_reference_timezone_as_z() -> Result<(), String> {
    // Given
    let runner = FakeCodexRunner::new(vec![FakeOutcome::WriteOutput(candidate_json().to_owned())]);
    let provider = CodexProvider::new(&runner);
    let evidence = [message(
        "chat-a",
        "msg-ambiguous-1",
        "Create a calendar event for July 2, 2026 at 3:30 PM.",
    )?];

    // When
    provider
        .extract_response(&evidence, "UTC")
        .map_err(|error| error.to_string())?;

    // Then
    let observation = runner.only_observation()?;
    assert_prompt_includes_normalized_time_contract(
        &observation.prompt_text,
        "2026-07-03T15:30:00Z",
    );
    assert!(!observation.prompt_text.contains("2026-07-03T15:30:00[UTC]"));
    Ok(())
}

fn assert_prompt_includes_normalized_time_contract(prompt: &str, configured_example: &str) {
    assert!(prompt.contains("normalized_time contract"));
    assert!(prompt.contains("YYYY-MM-DDTHH:MM:SS[Area/Location]"));
    assert!(prompt.contains("YYYY-MM-DDTHH:MM:SSZ"));
    assert!(prompt.contains("fixed-width"));
    assert!(prompt.contains("seconds are mandatory"));
    assert!(prompt.contains("safe IANA-style"));
    assert!(prompt.contains(configured_example));
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
}
