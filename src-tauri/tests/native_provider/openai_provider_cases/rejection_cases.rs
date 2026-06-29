use morrow_lib::native_bridge::{OpenAiHttpResponse, OpenAiProviderError};
use serde_json::json;

use crate::support::openai_provider::completed_response;
use crate::support::provider::candidate_json;

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
