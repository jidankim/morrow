use morrow_lib::native_bridge::CodexProvider;
use serde_json::Value;

use crate::codex_provider::{FakeCodexRunner, FakeOutcome};
use crate::provider::message;

#[test]
fn scheduling_intent_provider_prompt_guides_weak_calendar_phrasing_to_calendar_event_confidence(
) -> Result<(), String> {
    // Given
    let runner = FakeCodexRunner::new(vec![FakeOutcome::ClassifyWeakCalendarFromPrompt]);
    let provider = CodexProvider::new(&runner);
    let evidence = [message(
        "chat-a",
        "msg-weak-calendar",
        "Could we catch up Friday afternoon?",
    )?];

    // When
    let response = provider
        .extract_response(&evidence, "Asia/Seoul")
        .map_err(|error| error.to_string())?;

    // Then
    let candidate: Value =
        serde_json::from_str(response.raw_json()).map_err(|error| error.to_string())?;
    assert_eq!(candidate["kind"], "calendar_event");
    assert_eq!(candidate["title"], "Prompt-calibrated weak calendar");
    assert_eq!(candidate["anchor_message_guid"], "msg-weak-calendar");
    let confidence = candidate["confidence_millis"]
        .as_i64()
        .ok_or_else(|| "missing confidence_millis".to_owned())?;
    assert!((850..=1000).contains(&confidence));
    assert!(!runner
        .only_observation()?
        .prompt_text
        .contains("msg-weak-calendar"));
    Ok(())
}

#[test]
fn scheduling_intent_provider_prompt_keeps_weak_deadline_wording_as_task_reminder(
) -> Result<(), String> {
    // Given
    let runner = FakeCodexRunner::new(vec![FakeOutcome::ClassifyWeakTaskFromPrompt]);
    let provider = CodexProvider::new(&runner);
    let evidence = [message(
        "chat-a",
        "msg-weak-task",
        "Please follow up by July 25, 2026.",
    )?];

    // When
    let response = provider
        .extract_response(&evidence, "Asia/Seoul")
        .map_err(|error| error.to_string())?;

    // Then
    let candidate: Value =
        serde_json::from_str(response.raw_json()).map_err(|error| error.to_string())?;
    assert_eq!(candidate["kind"], "task_reminder");
    assert_eq!(candidate["title"], "Prompt-calibrated weak task");
    assert_eq!(
        candidate["normalized_time"],
        "2026-07-25T23:59:00[Asia/Seoul]"
    );
    assert_eq!(candidate["anchor_message_guid"], "msg-weak-task");
    let confidence = candidate["confidence_millis"]
        .as_i64()
        .ok_or_else(|| "missing confidence_millis".to_owned())?;
    assert!((700..=850).contains(&confidence));
    assert!(!runner
        .only_observation()?
        .prompt_text
        .contains("msg-weak-task"));
    Ok(())
}
