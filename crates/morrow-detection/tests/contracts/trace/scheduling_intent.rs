use std::error::Error;

use morrow_detection::{DetectionPipeline, SourceExcerptPolicy};
use morrow_diagnostics::{TraceDecision, TraceOperation};

use crate::support::{config, message, FakeProvider};

use super::helpers::{
    assert_trace_excludes, assert_trace_sequence, CollectingRecorder, UnavailableProvider,
};

#[test]
fn trace_records_scheduling_intent_weak_calendar_provider_route_reason(
) -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(Some(
        "{\"kind\":\"calendar_event\",\"title\":\"Catch up\",\
         \"confidence_millis\":720,\
         \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
         \"anchor_message_guid\":\"msg-weak-calendar-route-1\",\
         \"evidence_message_guids\":[\"msg-weak-calendar-route-1\"]}",
    ));
    let messages = vec![message(
        "chat-weak-calendar",
        "msg-weak-calendar-route-1",
        "MORROW_MESSAGE_CANARY catch up Friday afternoon?",
        false,
    )?];
    let mut config = config(550)?;
    config.source_excerpts = SourceExcerptPolicy::Hide;
    let recorder = CollectingRecorder::default();

    // When
    let report = DetectionPipeline::new(&provider).detect_with_trace(&messages, &config, &recorder);

    // Then
    assert_eq!(provider.calls(), 1);
    assert_eq!(report.candidates().count(), 1);
    let records = recorder.records()?;
    assert_parser_route_reason(&records, "parser_provider_route_weak_calendar");
    assert_provider_route_codes_preserved(
        &records,
        &[
            "provider_route",
            "provider_extract_success",
            "provider_schema_accepted",
            "confidence_meets_threshold",
            "candidate_materialized",
        ],
    );
    assert_trace_excludes(
        &records,
        &[
            "MORROW_MESSAGE_CANARY",
            "catch up Friday afternoon",
            "chat-weak-calendar",
            "msg-weak-calendar-route-1",
            "messages://chat-weak-calendar/msg-weak-calendar-route-1",
            "Catch up",
        ],
    )?;
    println!(
        "trace_records_scheduling_intent_weak_calendar_provider_route_reason reasons={:?}",
        reason_codes(&records)
    );
    Ok(())
}

#[test]
fn trace_records_scheduling_intent_task_deadline_provider_route_reason(
) -> Result<(), Box<dyn Error>> {
    // Given
    let provider = UnavailableProvider::default();
    let messages = vec![message(
        "chat-task-deadline",
        "msg-task-deadline-route-1",
        "MORROW_MESSAGE_CANARY send the renewal packet by July 25, 2026",
        false,
    )?];
    let mut config = config(550)?;
    config.source_excerpts = SourceExcerptPolicy::Hide;
    let recorder = CollectingRecorder::default();

    // When
    let report = DetectionPipeline::new(&provider).detect_with_trace(&messages, &config, &recorder);

    // Then
    assert_eq!(provider.calls(), 1);
    assert_eq!(report.candidates().count(), 1);
    let records = recorder.records()?;
    assert_parser_route_reason(&records, "parser_provider_route_task_deadline");
    assert_provider_route_codes_preserved(
        &records,
        &[
            "provider_route",
            "provider_unavailable",
            "candidate_materialized",
        ],
    );
    assert_trace_excludes(
        &records,
        &[
            "MORROW_MESSAGE_CANARY",
            "send the renewal packet",
            "chat-task-deadline",
            "msg-task-deadline-route-1",
            "messages://chat-task-deadline/msg-task-deadline-route-1",
        ],
    )?;
    println!(
        "trace_records_scheduling_intent_task_deadline_provider_route_reason reasons={:?}",
        reason_codes(&records)
    );
    Ok(())
}

#[test]
fn trace_records_scheduling_intent_ambiguous_calendar_provider_route_reason(
) -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(Some(
        "{\"kind\":\"calendar_event\",\"title\":\"Meet Friday afternoon\",\
         \"confidence_millis\":720,\
         \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
         \"anchor_message_guid\":\"msg-ambiguous-calendar-route-1\",\
         \"evidence_message_guids\":[\"msg-ambiguous-calendar-route-1\"]}",
    ));
    let messages = vec![message(
        "chat-ambiguous-calendar",
        "msg-ambiguous-calendar-route-1",
        "MORROW_MESSAGE_CANARY can we meet Friday afternoon?",
        true,
    )?];
    let mut config = config(550)?;
    config.source_excerpts = SourceExcerptPolicy::Hide;
    let recorder = CollectingRecorder::default();

    // When
    let report = DetectionPipeline::new(&provider).detect_with_trace(&messages, &config, &recorder);

    // Then
    assert_eq!(provider.calls(), 1);
    assert_eq!(report.candidates().count(), 1);
    let records = recorder.records()?;
    assert_parser_route_reason(&records, "parser_provider_route_ambiguous_calendar");
    assert_provider_route_codes_preserved(
        &records,
        &[
            "provider_route",
            "provider_extract_success",
            "provider_schema_accepted",
            "confidence_meets_threshold",
            "candidate_materialized",
        ],
    );
    assert_trace_excludes(
        &records,
        &[
            "MORROW_MESSAGE_CANARY",
            "can we meet Friday afternoon",
            "chat-ambiguous-calendar",
            "msg-ambiguous-calendar-route-1",
            "messages://chat-ambiguous-calendar/msg-ambiguous-calendar-route-1",
            "Meet Friday afternoon",
        ],
    )?;
    println!(
        "trace_records_scheduling_intent_ambiguous_calendar_provider_route_reason reasons={:?}",
        reason_codes(&records)
    );
    Ok(())
}

fn assert_parser_route_reason(records: &[morrow_diagnostics::TraceRecord], expected: &'static str) {
    assert_trace_sequence(
        &records[0..1],
        &[super::expectations::ExpectedTrace {
            operation: TraceOperation::ParserDecision,
            decision: Some(TraceDecision::ProviderRoute),
            outcome: morrow_diagnostics::TraceOutcome::Noop,
            reason_code: Some(expected),
        }],
    );
}

fn assert_provider_route_codes_preserved(
    records: &[morrow_diagnostics::TraceRecord],
    expected: &[&str],
) {
    let reasons = reason_codes(records);
    for expected_reason in expected {
        assert!(
            reasons.iter().any(|reason| reason == expected_reason),
            "missing preserved provider/result reason {expected_reason}; got {reasons:?}"
        );
    }
}

fn reason_codes(records: &[morrow_diagnostics::TraceRecord]) -> Vec<&str> {
    records
        .iter()
        .filter_map(|record| record.span.reason_code.as_deref())
        .collect()
}
