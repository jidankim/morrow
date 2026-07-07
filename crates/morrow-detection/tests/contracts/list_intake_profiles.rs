use std::error::Error;

use morrow_detection::{
    plan_list_intake_extraction, validate_list_intake_provider_output, DetectionPipeline,
    ListIntakeChatScope, ListIntakeConfidenceTier, ListIntakeRouteDecision,
    ListIntakeSchedulingDecision,
};

use crate::support::{config_with_profile_bare_quantity_lists, message, only_quiet, FakeProvider};

#[path = "list_intake_profiles_confidence.rs"]
mod confidence;
#[path = "list_intake_profile_fixture.rs"]
mod fixture;
#[path = "list_intake_profiles_private.rs"]
mod private;
#[path = "list_intake_profiles_window_category.rs"]
mod window_category;

use fixture::list_intake_profile;

#[test]
fn bare_quantity_list_does_not_enter_scheduling_provider_without_explicit_list_intake_profile(
) -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(Some(
        "{\"kind\":\"task_reminder\",\"title\":\"Daily list: 2 anchovies; 3 salmon\",\
         \"confidence_millis\":760,\
         \"normalized_time\":\"2026-06-26T23:59:00[Asia/Seoul]\",\
         \"anchor_message_guid\":\"msg-list-intake-boundary-1\",\
         \"evidence_message_guids\":[\"msg-list-intake-boundary-1\"]}",
    ));
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-list-intake-boundary-1",
        "2 anchovies, 3 salmon",
        false,
    )?];
    let config = config_with_profile_bare_quantity_lists(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    assert_eq!(
        provider.calls(),
        0,
        "bare quantity lists must not enter scheduling/provider routing without an explicit list-intake profile; legacy listReminderProfile is not an explicit list-intake profile"
    );
    let quiet = only_quiet(&report.outcomes)?;
    assert_eq!(quiet.reason, "deterministic_stop:no_scheduling_signal");
    println!(
        "bare_quantity_list_does_not_enter_scheduling_provider_without_explicit_list_intake_profile provider_calls={} quiet_reason={}",
        provider.calls(),
        quiet.reason
    );
    Ok(())
}

#[test]
fn list_intake_route_decisions_separate_scope_scheduling_and_capture() -> Result<(), Box<dyn Error>>
{
    // Given
    let evidence = message(
        "chat-1",
        "msg-list-intake-route-1",
        "2 anchovies, 3 salmon",
        false,
    )?;
    let mut profile = list_intake_profile();

    profile.enabled = false;
    assert_eq!(
        plan_list_intake_extraction(
            &profile,
            &evidence,
            ListIntakeSchedulingDecision::NotSchedulingOwned,
            "Asia/Seoul",
        ),
        ListIntakeRouteDecision::SkipProfileScope {
            reason: "disabled_profile"
        }
    );

    profile.enabled = true;
    profile.chat_scope = ListIntakeChatScope::SelectedChatIds(vec!["chat-other".to_owned()]);
    assert_eq!(
        plan_list_intake_extraction(
            &profile,
            &evidence,
            ListIntakeSchedulingDecision::NotSchedulingOwned,
            "Asia/Seoul",
        ),
        ListIntakeRouteDecision::SkipProfileScope {
            reason: "chat_scope_mismatch"
        }
    );

    profile.chat_scope = ListIntakeChatScope::AllSelectedChats;
    assert_eq!(
        plan_list_intake_extraction(
            &profile,
            &evidence,
            ListIntakeSchedulingDecision::SchedulingOwned,
            "Asia/Seoul",
        ),
        ListIntakeRouteDecision::SkipSchedulingOwned {
            reason: "scheduling_owned"
        }
    );

    profile.capture_from_scheduled_messages = true;
    match plan_list_intake_extraction(
        &profile,
        &evidence,
        ListIntakeSchedulingDecision::SchedulingOwned,
        "Asia/Seoul",
    ) {
        ListIntakeRouteDecision::ProviderExtraction(request) => {
            assert_eq!(request.profile_id, "list-intake-fishcount");
            assert_eq!(request.message_guid, "msg-list-intake-route-1");
            assert_eq!(request.reference_timezone, "Asia/Seoul");
        }
        other => {
            return Err(
                format!("expected provider extraction after capture opt-in, got {other:?}").into(),
            )
        }
    }
    println!(
        "list_intake_route_decisions_separate_scope_scheduling_and_capture disabled=skipProfileScope chat_mismatch=skipProfileScope scheduling_owned=skipSchedulingOwned capture=providerExtraction"
    );
    Ok(())
}

#[test]
fn list_intake_local_safety_rejects_private_and_unsupported_inputs() -> Result<(), Box<dyn Error>> {
    // Given
    let profile = list_intake_profile();
    let cases = [
        ("", "invalid_evidence"),
        ("https://example.com 2 salmon", "url_like"),
        ("2 a@example.com, 3 salmon", "email_like"),
        ("555-123-4567, 2 salmon", "phone_like"),
        ("2 salmon, owner@example.com", "email_like"),
        ("2 salmon, tel:+15555550103", "phone_like"),
        ("$12", "money_only"),
        ("+1 555 555 0103", "phone_like"),
        ("please remember the groceries", "unsupported_content"),
    ];

    for (excerpt, expected_reason) in cases {
        // When
        let evidence = message("chat-1", "msg-list-intake-safety", excerpt, false)?;
        let decision = plan_list_intake_extraction(
            &profile,
            &evidence,
            ListIntakeSchedulingDecision::NotSchedulingOwned,
            "Asia/Seoul",
        );

        // Then
        assert_eq!(
            decision,
            ListIntakeRouteDecision::SkipLocalSafety {
                reason: expected_reason
            },
            "excerpt {excerpt:?}"
        );
    }
    println!(
        "list_intake_local_safety_rejects_private_and_unsupported_inputs cases={}",
        cases.len()
    );
    Ok(())
}

#[test]
fn list_intake_provider_output_validates_without_raw_json_and_lowers_unknown_categories(
) -> Result<(), Box<dyn Error>> {
    // Given
    let profile = list_intake_profile();
    let evidence = message(
        "chat-1",
        "msg-list-intake-valid-1",
        "2 anchovies, 3 salmon",
        false,
    )?;

    // When
    let validated = validate_list_intake_provider_output(
        &profile,
        &evidence,
        "Asia/Seoul",
        "{\"matched\":true,\"confidence_millis\":920,\"items\":[\
         {\"name\":\"anchovies\",\"quantity\":2,\"unit\":null,\"categoryId\":\"seafood\",\
          \"evidenceText\":\"2 anchovies\"},\
         {\"name\":\"salmon\",\"quantity\":3,\"unit\":null,\"categoryId\":\"hacked-category\",\
          \"evidenceText\":\"3 salmon\"}]}",
    )?;

    // Then
    assert_eq!(validated.profile_id, "list-intake-fishcount");
    assert_eq!(validated.profile_version, "list-intake-v2");
    assert_eq!(validated.examples_hash, "list-intake-exampleshash");
    assert_eq!(validated.message_guid, "msg-list-intake-valid-1");
    assert_eq!(validated.window.window_timezone, "Asia/Seoul");
    assert_eq!(validated.provider_prompt_version, "list-intake-v1");
    assert_eq!(validated.items.len(), 2);
    assert_eq!(
        validated
            .items
            .first()
            .map(|item| item.category_id.as_str()),
        Some("seafood"),
        "expected first validated item to preserve the known seafood category"
    );
    assert_eq!(
        validated.items.get(1).map(|item| item.category_id.as_str()),
        Some("uncategorized"),
        "expected second validated item to fall back to uncategorized"
    );
    assert_eq!(validated.confidence_tier, ListIntakeConfidenceTier::Review);
    println!(
        "list_intake_provider_output_validates_without_raw_json_and_lowers_unknown_categories items={} tier={:?}",
        validated.items.len(),
        validated.confidence_tier
    );
    Ok(())
}

#[test]
fn list_intake_provider_output_rejects_malformed_and_hallucinated_rows(
) -> Result<(), Box<dyn Error>> {
    // Given
    let profile = list_intake_profile();
    let evidence = message(
        "chat-1",
        "msg-list-intake-invalid-1",
        "2 anchovies, 3 salmon",
        false,
    )?;
    let overlong_name = "x".repeat(81);
    let cases = [
        "{not json}".to_owned(),
        "{\"matched\":true,\"confidence_millis\":900,\"items\":[]}".to_owned(),
        format!(
            "{{\"matched\":true,\"confidence_millis\":900,\"items\":[{{\"name\":\"{overlong_name}\",\"quantity\":2,\"unit\":null,\"categoryId\":\"seafood\",\"evidenceText\":\"2 anchovies\"}}]}}"
        ),
        "{\"matched\":true,\"confidence_millis\":900,\"items\":[{\"name\":\"anchovies\",\"quantity\":0,\"unit\":null,\"categoryId\":\"seafood\",\"evidenceText\":\"2 anchovies\"}]}".to_owned(),
        "{\"matched\":true,\"confidence_millis\":900,\"items\":[{\"name\":\"anchovies\",\"quantity\":2,\"unit\":\"bad/unit\",\"categoryId\":\"seafood\",\"evidenceText\":\"2 anchovies\"}]}".to_owned(),
        "{\"matched\":true,\"confidence_millis\":900,\"items\":[{\"name\":\"lobster\",\"quantity\":2,\"unit\":null,\"categoryId\":\"seafood\",\"evidenceText\":\"2 lobster\"}]}".to_owned(),
    ];

    for provider_output in &cases {
        // When
        let result = validate_list_intake_provider_output(
            &profile,
            &evidence,
            "Asia/Seoul",
            provider_output,
        );

        // Then
        assert!(
            result.is_err(),
            "provider output unexpectedly validated: {provider_output}"
        );
    }
    println!(
        "list_intake_provider_output_rejects_malformed_and_hallucinated_rows cases={}",
        cases.len()
    );
    Ok(())
}
