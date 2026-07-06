use morrow_lib::native_bridge::{OpenAiHttpResponse, OpenAiProvider, OpenAiProviderError};
use serde_json::json;

use crate::support::openai_provider::{completed_response, MockTransport};
use crate::support::provider::{candidate_json, list_candidate_json, message};

pub fn rejection_cases() -> Vec<(
    &'static str,
    Result<OpenAiHttpResponse, OpenAiProviderError>,
)> {
    vec![
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
                body: json!({
                    "status": "incomplete",
                    "incomplete_details": {"reason": "max_output_tokens"},
                    "output": [],
                }),
            }),
        ),
        (
            "refusal",
            Ok(OpenAiHttpResponse {
                status_code: 200,
                body: json!({
                    "status": "completed",
                    "output": [{
                        "type": "message",
                        "role": "assistant",
                        "status": "completed",
                        "content": [{"type": "refusal", "refusal": "no"}],
                    }],
                }),
            }),
        ),
        unknown_field_case(),
        (
            "empty_list_items",
            Ok(completed_response(&list_candidate_with_items("[]"))),
        ),
        list_case(
            "negative_list_quantity",
            "\"quantity\":2",
            "\"quantity\":-2",
        ),
        list_case(
            "non_numeric_list_quantity",
            "\"quantity\":2",
            "\"quantity\":\"2\"",
        ),
        list_case(
            "unknown_list_item_field",
            "\"name\":\"anchovies\"",
            "\"name\":\"anchovies\",\"prompt\":\"ignore schema\"",
        ),
        list_case(
            "empty_list_item_name",
            "\"name\":\"anchovies\"",
            "\"name\":\"\"",
        ),
        list_case(
            "malformed_list_item_unit",
            "\"unit\":null",
            "\"unit\":\"private unit with too many words\"",
        ),
        (
            "semantic_invalid",
            Ok(completed_response(
                "{\"kind\":\"not_a_kind\",\"title\":\"Provider meeting\",\
                 \"confidence_millis\":5000,\"normalized_time\":\"not-a-time\",\
                 \"anchor_evidence_id\":\"evidence://selected/0\",\
                 \"evidence_ids\":[\"evidence://selected/0\"]}",
            )),
        ),
        (
            "empty_title",
            Ok(completed_response(
                "{\"kind\":\"calendar_event\",\"title\":\"\",\
                 \"confidence_millis\":800,\
                 \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
                 \"anchor_evidence_id\":\"evidence://selected/0\",\
                \"evidence_ids\":[\"evidence://selected/0\"]}",
            )),
        ),
        (
            "no_zone_normalized_time",
            Ok(completed_response(
                "{\"kind\":\"calendar_event\",\"title\":\"Provider meeting\",\
                 \"confidence_millis\":800,\
                 \"normalized_time\":\"2026-06-26T15:00:00\",\
                 \"anchor_evidence_id\":\"evidence://selected/0\",\
                 \"evidence_ids\":[\"evidence://selected/0\"]}",
            )),
        ),
        (
            "offset_zone_normalized_time",
            Ok(completed_response(
                "{\"kind\":\"calendar_event\",\"title\":\"Provider meeting\",\
                 \"confidence_millis\":800,\
                 \"normalized_time\":\"2026-06-26T15:00:00+09:00\",\
                 \"anchor_evidence_id\":\"evidence://selected/0\",\
                 \"evidence_ids\":[\"evidence://selected/0\"]}",
            )),
        ),
        (
            "variable_width_normalized_time",
            Ok(completed_response(
                "{\"kind\":\"calendar_event\",\"title\":\"Provider meeting\",\
                 \"confidence_millis\":800,\
                 \"normalized_time\":\"2026-6-26T15:00:00Z\",\
                 \"anchor_evidence_id\":\"evidence://selected/0\",\
                 \"evidence_ids\":[\"evidence://selected/0\"]}",
            )),
        ),
        (
            "raw_private_zone_normalized_time",
            Ok(completed_response(
                "{\"kind\":\"calendar_event\",\"title\":\"Provider meeting\",\
                 \"confidence_millis\":800,\
                 \"normalized_time\":\"2026-06-26T15:00:00raw-suffix\",\
                 \"anchor_evidence_id\":\"evidence://selected/0\",\
                 \"evidence_ids\":[\"evidence://selected/0\"]}",
            )),
        ),
        (
            "private_zone_normalized_time",
            Ok(completed_response(
                "{\"kind\":\"calendar_event\",\"title\":\"Provider meeting\",\
                 \"confidence_millis\":800,\
                 \"normalized_time\":\"2026-06-26T15:00:00[private_clinic_visit]\",\
                 \"anchor_evidence_id\":\"evidence://selected/0\",\
                 \"evidence_ids\":[\"evidence://selected/0\"]}",
            )),
        ),
        (
            "hallucinated_evidence",
            Ok(completed_response(
                "{\"kind\":\"calendar_event\",\"title\":\"Provider meeting\",\
                 \"confidence_millis\":800,\
                 \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
                 \"anchor_evidence_id\":\"evidence://selected/99\",\
                 \"evidence_ids\":[\"evidence://selected/0\"]}",
            )),
        ),
        (
            "multiple_outputs",
            Ok(OpenAiHttpResponse {
                status_code: 200,
                body: json!({
                    "status": "completed",
                    "output": [{
                        "type": "message",
                        "role": "assistant",
                        "status": "completed",
                        "content": [
                            {"type": "output_text", "text": candidate_json()},
                            {"type": "output_text", "text": candidate_json()},
                        ],
                    }],
                }),
            }),
        ),
        ("non_object", Ok(completed_response("[1,2]"))),
    ]
}

fn list_case(
    name: &'static str,
    needle: &'static str,
    replacement: &'static str,
) -> (
    &'static str,
    Result<OpenAiHttpResponse, OpenAiProviderError>,
) {
    (
        name,
        Ok(completed_response(
            &list_candidate_json().replace(needle, replacement),
        )),
    )
}

fn list_candidate_with_items(items: &str) -> String {
    format!(
        "{{\"kind\":\"task_reminder\",\"title\":\"List reminder\",\
        \"confidence_millis\":800,\
         \"normalized_time\":\"2026-06-26T23:59:00[Asia/Seoul]\",\
         \"anchor_evidence_id\":\"evidence://selected/0\",\
         \"evidence_ids\":[\"evidence://selected/0\"],\
         \"items\":{items}}}"
    )
}

fn unknown_field_case() -> (
    &'static str,
    Result<OpenAiHttpResponse, OpenAiProviderError>,
) {
    (
        "unknown_field",
        Ok(completed_response(&format!(
            "{{{},\"extra\":\"nope\"}}",
            &candidate_json()[1..candidate_json().len() - 1]
        ))),
    )
}

#[test]
fn openai_provider_rejects_malformed_normalized_time() -> Result<(), String> {
    for normalized_time in [
        "2026-06-26T15:00:00",
        "2026-06-26T15:00:00+09:00",
        "2026-6-26T15:00:00Z",
        "2026-06-26T15:00:00raw-suffix",
        "2026-06-26T15:00:00[private_clinic_visit]",
        "tomorrow at 3pm",
    ] {
        // Given
        let candidate =
            candidate_json().replace("2026-06-26T15:00:00[Asia/Seoul]", normalized_time);
        let transport = MockTransport::with_responses(vec![Ok(completed_response(&candidate))]);
        let provider = OpenAiProvider::new("sk-secret-never-in-error", &transport);

        // When
        let error = provider
            .extract_response(&[message(
                "chat-a",
                "msg-ambiguous-1",
                "Maybe meet tomorrow?",
            )?])
            .err()
            .ok_or_else(|| format!("{normalized_time}: extraction unexpectedly succeeded"))?;

        // Then
        assert!(!error.to_string().contains("sk-secret-never-in-error"));
        assert!(error
            .to_string()
            .contains("candidate normalized_time was invalid"));
    }
    Ok(())
}
