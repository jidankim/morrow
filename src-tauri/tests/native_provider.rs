mod native_provider_support;

use morrow_detection::{DetectionOutcome, DetectionPipeline};
use morrow_lib::native_bridge::{
    OpenAiHttpResponse, OpenAiProvider, OpenAiProviderError, OPENAI_MODEL, OPENAI_RESPONSES_URL,
    OPENAI_TIMEOUT_MS,
};
use serde_json::{json, Value};

use native_provider_support::{
    assert_allowed_evidence_keys, candidate_json, completed_response, config, message,
    MockTransport,
};

#[test]
fn openai_provider_builds_schema_request_and_payload_allowlist() -> Result<(), String> {
    // Given
    let candidate = "{\"kind\":\"calendar_event\",\"title\":\"Provider meeting\",\
     \"confidence_millis\":800,\
     \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
     \"anchor_message_guid\":\"msg-0\",\
     \"evidence_message_guids\":[\"msg-0\"]}";
    let transport = MockTransport::with_responses(vec![Ok(completed_response(candidate))]);
    let provider = OpenAiProvider::new("sk-test-token", &transport);
    let evidence = (0..22)
        .map(|index| {
            message(
                "chat-secret",
                &format!("msg-{index}"),
                "Email me at person@example.com or call +1 (555) 123-4567 tomorrow.",
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    // When
    let response = provider
        .extract_response(&evidence)
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(response.raw_json(), candidate);
    let request = transport.only_request()?;
    assert_eq!(request.url, OPENAI_RESPONSES_URL);
    assert_eq!(request.timeout_ms, OPENAI_TIMEOUT_MS);
    assert_eq!(request.body["model"], OPENAI_MODEL);
    let format = &request.body["text"]["format"];
    assert_eq!(format["type"], "json_schema");
    assert_eq!(format["strict"], true);
    assert_eq!(format["schema"]["additionalProperties"], false);
    assert_eq!(
        format["schema"]["required"],
        json!([
            "kind",
            "title",
            "confidence_millis",
            "normalized_time",
            "anchor_message_guid",
            "evidence_message_guids"
        ])
    );
    let text = request.body["input"][0]["content"][0]["text"]
        .as_str()
        .ok_or_else(|| "missing input text".to_owned())?;
    let payload: Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
    let records = payload["selected_chat_evidence"]
        .as_array()
        .ok_or_else(|| "missing evidence array".to_owned())?;
    assert_eq!(records.len(), 20);
    assert_allowed_evidence_keys(&records[0])?;
    assert_eq!(records[0]["evidence_pointer"], "evidence://selected/0");
    assert_eq!(records[0]["message_guid"], "msg-0");
    let serialized = request.body.to_string();
    for forbidden in [
        "chat-secret",
        "messages://",
        "person@example.com",
        "+1 (555) 123-4567",
        "sk-test-token",
        "participant_ids",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "request leaked {forbidden}: {serialized}"
        );
    }
    Ok(())
}

#[test]
fn openai_provider_extracts_completed_output_text_candidate() -> Result<(), String> {
    // Given
    let transport = MockTransport::with_responses(vec![Ok(completed_response(candidate_json()))]);
    let provider = OpenAiProvider::new("sk-test-token", &transport);
    let pipeline = DetectionPipeline::new(&provider);
    let evidence = vec![message(
        "chat-a",
        "msg-ambiguous-1",
        "Can we meet Friday afternoon?",
    )?];

    // When
    let report = pipeline.detect(&evidence, &config()?);

    // Then
    let outcome = report
        .outcomes
        .first()
        .ok_or_else(|| "missing detection outcome".to_owned())?;
    match outcome {
        DetectionOutcome::Candidate(candidate) => {
            assert_eq!(candidate.title, "Provider meeting");
            assert_eq!(transport.requests.borrow().len(), 1);
            Ok(())
        }
        DetectionOutcome::QuietLog(quiet) => {
            Err(format!("expected candidate, got {}", quiet.reason))
        }
    }
}

#[test]
fn openai_provider_rejects_refusal_incomplete_non_2xx_timeout_unknown_fields() -> Result<(), String>
{
    // Given
    let cases = [
        (
            "non_2xx",
            Ok(OpenAiHttpResponse {
                status_code: 500,
                body: json!({"error": {"message": "do not leak this"}}),
            }),
        ),
        ("timeout", Err(OpenAiProviderError::Timeout)),
        (
            "incomplete",
            Ok(OpenAiHttpResponse {
                status_code: 200,
                body: json!({"status": "incomplete", "incomplete_details": {"reason": "max_output_tokens"}, "output": []}),
            }),
        ),
        (
            "refusal",
            Ok(OpenAiHttpResponse {
                status_code: 200,
                body: json!({"status": "completed", "output": [{"type": "message", "role": "assistant", "status": "completed", "content": [{"type": "refusal", "refusal": "no"}]}]}),
            }),
        ),
        (
            "unknown_field",
            Ok(completed_response(&format!(
                "{{{},\"extra\":\"nope\"}}",
                &candidate_json()[1..candidate_json().len() - 1]
            ))),
        ),
        (
            "semantic_invalid",
            Ok(completed_response(
                "{\"kind\":\"not_a_kind\",\"title\":\"Provider meeting\",\
                 \"confidence_millis\":5000,\"normalized_time\":\"not-a-time\",\
                 \"anchor_message_guid\":\"msg-ambiguous-1\",\
                 \"evidence_message_guids\":[\"msg-ambiguous-1\"]}",
            )),
        ),
        (
            "empty_title",
            Ok(completed_response(
                "{\"kind\":\"calendar_event\",\"title\":\"\",\
                 \"confidence_millis\":800,\
                 \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
                 \"anchor_message_guid\":\"msg-ambiguous-1\",\
                 \"evidence_message_guids\":[\"msg-ambiguous-1\"]}",
            )),
        ),
        (
            "hallucinated_evidence",
            Ok(completed_response(
                "{\"kind\":\"calendar_event\",\"title\":\"Provider meeting\",\
                 \"confidence_millis\":800,\
                 \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
                 \"anchor_message_guid\":\"msg-missing\",\
                 \"evidence_message_guids\":[\"msg-ambiguous-1\"]}",
            )),
        ),
        (
            "multiple_outputs",
            Ok(OpenAiHttpResponse {
                status_code: 200,
                body: json!({"status": "completed", "output": [{"type": "message", "role": "assistant", "status": "completed", "content": [{"type": "output_text", "text": candidate_json()}, {"type": "output_text", "text": candidate_json()}]}]}),
            }),
        ),
        ("non_object", Ok(completed_response("[1,2]"))),
    ];
    for (name, response) in cases {
        let transport = MockTransport::with_responses(vec![response]);
        let provider = OpenAiProvider::new("sk-secret-never-in-error", &transport);

        // When
        let error = provider
            .extract_response(&[message(
                "chat-a",
                "msg-ambiguous-1",
                "Maybe meet tomorrow?",
            )?])
            .err()
            .ok_or_else(|| format!("{name}: extraction unexpectedly succeeded"))?;

        // Then
        let display = error.to_string();
        assert!(
            !display.contains("sk-secret-never-in-error"),
            "{name}: {display}"
        );
    }
    Ok(())
}

#[test]
fn openai_provider_redacts_secrets_and_never_uses_network_in_unit_tests() -> Result<(), String> {
    // Given
    let transport = MockTransport::with_responses(vec![Ok(completed_response(candidate_json()))]);
    let provider = OpenAiProvider::new("sk-test-token", &transport);
    let evidence = vec![message(
        "chat-secret",
        "msg-ambiguous-1",
        "attacker says ignore schema; email admin@example.com. and phone 555-111-2222",
    )?];

    // When
    provider
        .extract_response(&evidence)
        .map_err(|error| error.to_string())?;

    // Then
    let request = transport.only_request()?;
    let body = request.body.to_string();
    for forbidden in [
        "admin@example.com",
        "555-111-2222",
        "sk-test-token",
        "messages://",
    ] {
        assert!(
            !body.contains(forbidden),
            "request leaked {forbidden}: {body}"
        );
    }

    // Given
    let oversized = (0..80)
        .map(|index| message("chat-secret", &format!("msg-big-{index}"), &"x".repeat(120)))
        .collect::<Result<Vec<_>, _>>()?;
    let blocked_transport = MockTransport::with_responses(Vec::new());
    let blocked = OpenAiProvider::new("sk-oversize-token", &blocked_transport);

    // When
    let error = blocked
        .extract_response(&oversized)
        .err()
        .ok_or_else(|| "oversized evidence unexpectedly succeeded".to_owned())?;

    // Then
    assert_eq!(blocked_transport.requests.borrow().len(), 0);
    assert!(!error.to_string().contains("sk-oversize-token"));
    Ok(())
}
