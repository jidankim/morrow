use std::error::Error;

use morrow_detection::DetectionPipeline;
use morrow_storage::CandidateKind;

use crate::support::{
    config, config_with_profile_bare_quantity_lists, message, only_candidate, only_quiet,
    FakeProvider,
};

#[test]
fn bare_quantity_list_stops_before_provider_when_profile_disabled() -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(None);
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-bare-list-disabled-1",
        "2 anchovies, 3 salmon",
        false,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    assert_eq!(provider.calls(), 0);
    let quiet = only_quiet(&report.outcomes)?;
    assert_eq!(quiet.reason, "deterministic_stop:no_scheduling_signal");
    println!(
        "bare_quantity_list_stops_before_provider_when_profile_disabled provider_calls={} quiet_reason={}",
        provider.calls(),
        quiet.reason
    );
    Ok(())
}

#[test]
fn am_pm_time_like_bare_list_stops_before_provider_when_profile_disabled(
) -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(None);
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-bare-list-disabled-ampm-1",
        "9 am, 10 pm",
        false,
    )?];
    let config = config(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    assert_eq!(provider.calls(), 0);
    let quiet = only_quiet(&report.outcomes)?;
    assert_eq!(quiet.reason, "deterministic_stop:no_scheduling_signal");
    println!(
        "am_pm_time_like_bare_list_stops_before_provider_when_profile_disabled provider_calls={} quiet_reason={}",
        provider.calls(),
        quiet.reason
    );
    Ok(())
}

#[test]
fn bare_quantity_list_routes_to_provider_when_profile_enabled() -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(Some(
        "{\"kind\":\"task_reminder\",\"title\":\"Daily list: 2 anchovies; 3 salmon\",\
         \"confidence_millis\":760,\
         \"normalized_time\":\"2026-06-26T23:59:00[Asia/Seoul]\",\
         \"anchor_message_guid\":\"msg-bare-list-enabled-1\",\
         \"evidence_message_guids\":[\"msg-bare-list-enabled-1\"]}",
    ));
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-bare-list-enabled-1",
        "2 anchovies, 3 salmon",
        false,
    )?];
    let config = config_with_profile_bare_quantity_lists(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    assert_eq!(provider.calls(), 1);
    let candidate = only_candidate(&report.outcomes)?;
    assert_eq!(candidate.kind, CandidateKind::TaskReminder);
    assert_eq!(candidate.title, "Daily list: 2 anchovies; 3 salmon");
    println!(
        "bare_quantity_list_routes_to_provider_when_profile_enabled provider_calls={} candidate_kind={:?}",
        provider.calls(),
        candidate.kind
    );
    Ok(())
}

#[test]
fn malformed_bare_quantity_lists_do_not_route_to_provider() -> Result<(), Box<dyn Error>> {
    for (message_guid, excerpt) in [
        ("msg-bare-list-decimal-1", "2.5 anchovies, 3 salmon"),
        ("msg-bare-list-range-1", "2-3 anchovies, 4 salmon"),
        ("msg-bare-list-date-1", "2026-07-25, 2026-07-26"),
        ("msg-bare-list-time-1", "9:30 anchovies, 10 salmon"),
        ("msg-bare-list-ampm-time-1", "9 am, 10 pm"),
        ("msg-bare-list-phone-1", "555-123-4567, 2 salmon"),
        ("msg-bare-list-url-1", "2 https://example.com, 3 salmon"),
        ("msg-bare-list-bare-domain-1", "2 example.com, 3 salmon"),
        ("msg-bare-list-www-domain-1", "2 www.example.com, 3 salmon"),
        ("msg-bare-list-email-1", "2 a@example.com, 3 salmon"),
        ("msg-bare-list-currency-1", "$2 anchovies, 3 salmon"),
        ("msg-bare-list-empty-name-1", "2, 3 salmon"),
        ("msg-bare-list-one-off-1", "2 anchovies"),
        ("msg-bare-list-arbitrary-1", "12345 anchovies, 3 salmon"),
    ] {
        // Given
        let provider = FakeProvider::new(None);
        let pipeline = DetectionPipeline::new(&provider);
        let messages = vec![message("chat-1", message_guid, excerpt, false)?];
        let config = config_with_profile_bare_quantity_lists(550)?;

        // When
        let report = pipeline.detect(&messages, &config);

        // Then
        assert_eq!(provider.calls(), 0, "excerpt should reject: {excerpt}");
        let quiet = only_quiet(&report.outcomes)?;
        assert_eq!(quiet.reason, "deterministic_stop:no_scheduling_signal");
        println!(
            "malformed_bare_quantity_lists_do_not_route_to_provider excerpt={excerpt:?} provider_calls={} quiet_reason={}",
            provider.calls(),
            quiet.reason
        );
    }
    Ok(())
}
