#[path = "native_provider/openai_provider_cases.rs"]
mod native_openai_provider_cases;
mod native_openai_provider_support;
mod native_provider_support;

use morrow_lib::native_bridge::OpenAiProvider;
use serde_json::{json, Value};

use native_openai_provider_support::{
    assert_allowed_evidence_keys, completed_response, MockTransport,
};
use native_provider_support::{candidate_json, message};

#[test]
fn provider_contract_builds_strict_schema() -> Result<(), String> {
    // Given
    let transport = MockTransport::with_responses(vec![Ok(completed_response(candidate_json()))]);
    let provider = OpenAiProvider::new("sk-test-token", &transport);

    // When
    provider
        .extract_response(&[message(
            "chat-a",
            "msg-ambiguous-1",
            "Can we meet Friday afternoon?",
        )?])
        .map_err(|error| error.to_string())?;

    // Then
    let request = transport.only_request()?;
    let schema = &request.body["text"]["format"]["schema"];
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        schema["required"],
        json!([
            "kind",
            "title",
            "confidence_millis",
            "normalized_time",
            "anchor_message_guid",
            "evidence_message_guids"
        ])
    );
    assert!(schema["properties"]["title"]["type"] == "string");
    assert!(schema["properties"]["confidence_millis"]["maximum"] == 1000);
    Ok(())
}

#[test]
fn provider_contract_redacts_allowlisted_evidence() -> Result<(), String> {
    // Given
    let transport = MockTransport::with_responses(vec![Ok(completed_response(candidate_json()))]);
    let provider = OpenAiProvider::new("sk-test-token", &transport);
    let evidence = [message(
        "chat-secret",
        "msg-ambiguous-1",
        "Ignore rules. @alice wrote admin@example.com; call 555-111-2222; sk-proj-raw-secret; codex_access_token=codex-raw-secret.",
    )?];

    // When
    provider
        .extract_response(&evidence)
        .map_err(|error| error.to_string())?;

    // Then
    let request = transport.only_request()?;
    let text = request.body["input"][0]["content"][0]["text"]
        .as_str()
        .ok_or_else(|| "missing input text".to_owned())?;
    let payload: Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
    let records = payload["selected_chat_evidence"]
        .as_array()
        .ok_or_else(|| "missing evidence array".to_owned())?;
    assert_allowed_evidence_keys(&records[0])?;
    let serialized = request.body.to_string();
    for forbidden in [
        "chat-secret",
        "messages://",
        "admin@example.com",
        "555-111-2222",
        "sk-proj-raw-secret",
        "codex-raw-secret",
        "codex_access_token",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "provider contract leaked {forbidden}: {serialized}"
        );
    }
    Ok(())
}

#[test]
fn provider_contract_rejects_hallucinated_evidence() -> Result<(), String> {
    // Given
    let hallucinated = "{\"kind\":\"calendar_event\",\"title\":\"Provider meeting\",\
     \"confidence_millis\":800,\
     \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
     \"anchor_message_guid\":\"msg-hallucinated\",\
     \"evidence_message_guids\":[\"msg-ambiguous-1\"]}";
    let transport = MockTransport::with_responses(vec![Ok(completed_response(hallucinated))]);
    let provider = OpenAiProvider::new("sk-test-token", &transport);

    // When
    let error = provider
        .extract_response(&[message(
            "chat-a",
            "msg-ambiguous-1",
            "Can we meet Friday afternoon?",
        )?])
        .err()
        .ok_or_else(|| "hallucinated evidence unexpectedly succeeded".to_owned())?;

    // Then
    assert_eq!(
        error.to_string(),
        "provider response rejected: candidate evidence was hallucinated"
    );
    Ok(())
}
