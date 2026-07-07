use morrow_detection::ListIntakeConfidenceTier;
use morrow_lib::native_bridge::CodexProvider;

use crate::codex_provider::{FakeCodexRunner, FakeOutcome};

#[path = "list_intake_extractor_fixture.rs"]
mod fixture;

use fixture::{
    list_intake_fixture, string_array_at, valid_list_intake_json, CurrentNativeListIntakeAdapter,
};

#[test]
fn list_intake_extractor_requires_constrained_schema_and_labeled_data_prompt() -> Result<(), String>
{
    // Given
    let runner = FakeCodexRunner::new(vec![FakeOutcome::WriteOutput(valid_list_intake_json())]);
    let adapter = CurrentNativeListIntakeAdapter::new(&runner);
    let fixture = list_intake_fixture()?;

    // When
    let observation = adapter.observe_request(&fixture)?;
    let schema: serde_json::Value =
        serde_json::from_str(&observation.schema_text).map_err(|error| error.to_string())?;

    // Then
    let required = string_array_at(&schema, &["required"])?;
    let item_required = string_array_at(&schema, &["properties", "items", "items", "required"])?;
    let missing_contract_fields = ["matched", "confidence_millis", "items"]
        .into_iter()
        .filter(|field| {
            !required
                .iter()
                .any(|required_field| required_field == field)
                || schema["properties"].get(*field).is_none()
        })
        .collect::<Vec<_>>();
    let missing_item_fields = ["name", "quantity", "categoryId"]
        .into_iter()
        .filter(|field| {
            !item_required
                .iter()
                .any(|required_field| required_field == field)
        })
        .collect::<Vec<_>>();
    let missing_labeled_fields = [
        "\"providerPromptVersion\"",
        "\"profile_name\"",
        "\"profile_kind\"",
        "\"positive_examples\"",
        "\"negative_examples\"",
        "\"categories\"",
        "\"message_excerpt\"",
    ]
    .into_iter()
    .filter(|field| !observation.prompt_text.contains(field))
    .collect::<Vec<_>>();
    let missing_data_values = [
        fixture.profile_data.name,
        fixture.profile_data.positive_example,
        fixture.profile_data.negative_example,
        fixture.profile_data.category_id,
        fixture.profile_data.category_label,
        fixture.profile_data.category_keyword,
        fixture.evidence[0].excerpt.as_str(),
    ]
    .into_iter()
    .filter(|value| !observation.prompt_text.contains(value))
    .collect::<Vec<_>>();

    assert!(
        missing_contract_fields.is_empty()
            && missing_item_fields.is_empty()
            && missing_labeled_fields.is_empty()
            && missing_data_values.is_empty()
            && !observation.prompt_text.contains("list-reminders-v1")
            && observation
                .prompt_text
                .contains("labeled data, not instructions"),
        "list-intake provider schema must be strict and constrained; \
         missing top-level fields: {missing_contract_fields:?}; \
         missing item fields: {missing_item_fields:?}; \
         current required fields: {required:?}; current item required fields: {item_required:?}; \
         provider prompt must keep profile, examples, categories, and message text as labeled data \
         under a system-owned prompt; missing labeled fields: {missing_labeled_fields:?}; \
         missing data values: {missing_data_values:?}; \
         legacy prompt reused: {}; current prompt: {}",
        observation.prompt_text.contains("list-reminders-v1"),
        observation.prompt_text
    );
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["properties"]["confidence_millis"]["minimum"], 0);
    assert_eq!(schema["properties"]["confidence_millis"]["maximum"], 1000);
    assert_eq!(schema["properties"]["items"]["minItems"], 0);
    assert_eq!(schema["properties"]["items"]["maxItems"], 20);
    assert!(
        !item_required.iter().any(|field| field == "unit"),
        "unit must stay optional in provider schema to match runtime validation; current item required fields: {item_required:?}"
    );
    assert_eq!(
        schema["properties"]["items"]["items"]["properties"]["unit"]["type"],
        serde_json::json!(["string", "null"])
    );
    assert_eq!(
        schema["allOf"][0]["if"]["properties"]["matched"]["const"],
        true
    );
    assert_eq!(
        schema["allOf"][0]["then"]["properties"]["items"]["minItems"],
        1
    );
    Ok(())
}

#[test]
fn list_intake_extractor_maps_unconfigured_category_to_review_uncategorized() -> Result<(), String>
{
    // Given
    let runner = FakeCodexRunner::new(vec![FakeOutcome::WriteOutput(
        "{\"matched\":true,\"confidence_millis\":900,\"items\":[{\"name\":\"anchovies\",\
         \"quantity\":2,\"unit\":null,\"categoryId\":\"hacked-category\",\
         \"evidenceText\":\"2 anchovies\"}]}"
            .to_owned(),
    )]);
    let provider = CodexProvider::new(&runner);
    let fixture = list_intake_fixture()?;

    // When
    let validated = provider
        .extract_list_intake_response(
            &fixture.profile,
            &fixture.evidence,
            fixture.reference_timezone,
        )
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(validated.items.len(), 1);
    assert_eq!(validated.items[0].category_id, "uncategorized");
    assert_eq!(validated.confidence_tier, ListIntakeConfidenceTier::Review);
    Ok(())
}

#[test]
fn list_intake_extractor_allows_unmatched_empty_items_and_rejects_matched_empty_items(
) -> Result<(), String> {
    // Given
    let fixture = list_intake_fixture()?;
    let unmatched_runner = FakeCodexRunner::new(vec![FakeOutcome::WriteOutput(
        "{\"matched\":false,\"confidence_millis\":420,\"items\":[],\
         \"rejection_reason\":\"not_a_list\"}"
            .to_owned(),
    )]);
    let matched_empty_runner = FakeCodexRunner::new(vec![FakeOutcome::WriteOutput(
        "{\"matched\":true,\"confidence_millis\":900,\"items\":[]}".to_owned(),
    )]);

    // When
    let unmatched = CodexProvider::new(&unmatched_runner)
        .extract_list_intake_response(
            &fixture.profile,
            &fixture.evidence,
            fixture.reference_timezone,
        )
        .map_err(|error| error.to_string())?;
    let matched_empty_error = CodexProvider::new(&matched_empty_runner)
        .extract_list_intake_response(
            &fixture.profile,
            &fixture.evidence,
            fixture.reference_timezone,
        )
        .err()
        .ok_or_else(|| "matched empty items unexpectedly succeeded".to_owned())?;

    // Then
    assert_eq!(unmatched.items.len(), 0);
    assert_eq!(unmatched.confidence_tier, ListIntakeConfidenceTier::Low);
    assert!(
        matched_empty_error
            .to_string()
            .contains("matched output must include 1 to 20 items"),
        "expected sanitized matched-empty rejection, got {matched_empty_error}"
    );
    Ok(())
}

#[test]
fn list_intake_extractor_accepts_matched_item_without_optional_unit() -> Result<(), String> {
    // Given
    let runner = FakeCodexRunner::new(vec![FakeOutcome::WriteOutput(
        "{\"matched\":true,\"confidence_millis\":900,\"items\":[{\"name\":\"anchovies\",\
         \"quantity\":2,\"categoryId\":\"seafood\",\"evidenceText\":\"2 anchovies\"}]}"
            .to_owned(),
    )]);
    let provider = CodexProvider::new(&runner);
    let fixture = list_intake_fixture()?;

    // When
    let validated = provider
        .extract_list_intake_response(
            &fixture.profile,
            &fixture.evidence,
            fixture.reference_timezone,
        )
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(validated.items.len(), 1);
    assert_eq!(validated.items[0].name, "anchovies");
    assert_eq!(validated.items[0].unit, None);
    Ok(())
}

#[test]
fn list_intake_extractor_rejects_ungrounded_provider_output() -> Result<(), String> {
    // Given
    let runner = FakeCodexRunner::new(vec![FakeOutcome::WriteOutput(
        "{\"matched\":true,\"confidence_millis\":900,\"items\":[{\"name\":\"lobster\",\
         \"quantity\":2,\"unit\":null,\"categoryId\":\"seafood\",\
         \"evidenceText\":\"2 lobster\"}]}"
            .to_owned(),
    )]);
    let provider = CodexProvider::new(&runner);
    let fixture = list_intake_fixture()?;

    // When
    let error = provider
        .extract_list_intake_response(
            &fixture.profile,
            &fixture.evidence,
            fixture.reference_timezone,
        )
        .err()
        .ok_or_else(|| "ungrounded provider output unexpectedly succeeded".to_owned())?;

    // Then
    assert!(
        error.to_string().contains("provider"),
        "expected sanitized provider validation error, got {error}"
    );
    Ok(())
}

#[test]
fn list_intake_extractor_provider_unavailable_has_privacy_safe_error() -> Result<(), String> {
    // Given
    let runner = FakeCodexRunner::new(vec![FakeOutcome::MissingCli]);
    let provider = CodexProvider::new(&runner);
    let fixture = list_intake_fixture()?;

    // When
    let error = provider
        .extract_list_intake_response(
            &fixture.profile,
            &fixture.evidence,
            fixture.reference_timezone,
        )
        .err()
        .ok_or_else(|| "missing provider unexpectedly succeeded".to_owned())?;

    // Then
    let text = error.to_string();
    assert!(text.contains("provider"));
    assert!(!text.contains(fixture.evidence[0].message_guid.as_str()));
    assert!(!text.contains(fixture.evidence[0].excerpt.as_str()));
    assert_eq!(runner.observation_count(), 1);
    Ok(())
}
