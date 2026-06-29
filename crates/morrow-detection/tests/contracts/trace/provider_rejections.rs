use std::error::Error;

use morrow_detection::{DetectionPipeline, SourceExcerptPolicy};

use crate::support::{config, message, only_quiet, FakeProvider};

use super::expectations::{schema_rejection_expectations, threshold_rejection_expectations};
use super::helpers::{
    assert_provider_metadata_prefix, assert_trace_excludes, assert_trace_sequence,
    CollectingRecorder,
};

#[test]
fn trace_records_schema_rejection_without_raw_provider_json() -> Result<(), Box<dyn Error>> {
    let provider = FakeProvider::new(Some(
        "{\"kind\":\"calendar_event\",\"title\":\"MORROW_PROVIDER_TITLE_CANARY_FULL\",\
         \"confidence_millis\":800,\
         \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
         \"anchor_message_guid\":\"msg-made-up\",\
         \"evidence_message_guids\":[\"msg-made-up\"]}",
    ));
    let messages = vec![message(
        "chat-1",
        "msg-real-1",
        "MORROW_MESSAGE_CANARY meet Friday afternoon?",
        true,
    )?];
    let mut config = config(550)?;
    config.source_excerpts = SourceExcerptPolicy::Hide;
    let recorder = CollectingRecorder::default();

    let report = DetectionPipeline::new(&provider).detect_with_trace(&messages, &config, &recorder);

    let quiet = only_quiet(&report.outcomes)?;
    assert_eq!(quiet.reason, "provider_hallucinated_evidence");
    assert_eq!(provider.calls(), 1);
    let records = recorder.records()?;
    assert_trace_sequence(
        &records,
        &schema_rejection_expectations("provider_hallucinated_evidence"),
    );
    assert_provider_metadata_prefix(&records, 4);
    assert_trace_excludes(
        &records,
        &[
            "MORROW_PROVIDER_TITLE_CANARY_FULL",
            "MORROW_MESSAGE_CANARY",
            "msg-made-up",
            "raw_json",
            "provider_json",
        ],
    )?;
    Ok(())
}

#[test]
fn trace_records_parser_provider_conflict_decision() -> Result<(), Box<dyn Error>> {
    let provider = FakeProvider::new(Some(
        "{\"kind\":\"calendar_event\",\"title\":\"MORROW_PROVIDER_TITLE_CANARY_FULL\",\
         \"confidence_millis\":800,\
         \"normalized_time\":\"2026-06-28T16:00:00[Asia/Seoul]\",\
         \"anchor_message_guid\":\"msg-conflict-1\",\
         \"evidence_message_guids\":[\"msg-conflict-1\"]}",
    ));
    let messages = vec![message(
        "chat-1",
        "msg-conflict-1",
        "MORROW_MESSAGE_CANARY maybe meet 2026-06-28 15:00?",
        true,
    )?];
    let mut config = config(550)?;
    config.source_excerpts = SourceExcerptPolicy::Hide;
    let recorder = CollectingRecorder::default();

    let report = DetectionPipeline::new(&provider).detect_with_trace(&messages, &config, &recorder);

    let quiet = only_quiet(&report.outcomes)?;
    assert_eq!(quiet.reason, "parser_provider_time_conflict");
    let records = recorder.records()?;
    assert_trace_sequence(
        &records,
        &schema_rejection_expectations("parser_provider_time_conflict"),
    );
    assert_trace_excludes(
        &records,
        &["MORROW_PROVIDER_TITLE_CANARY_FULL", "MORROW_MESSAGE_CANARY"],
    )?;
    Ok(())
}

#[test]
fn trace_records_threshold_rejection_decision() -> Result<(), Box<dyn Error>> {
    let provider = FakeProvider::new(Some(
        "{\"kind\":\"calendar_event\",\"title\":\"MORROW_PROVIDER_TITLE_CANARY_FULL\",\
         \"confidence_millis\":549,\
         \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
         \"anchor_message_guid\":\"msg-ambiguous-1\",\
         \"evidence_message_guids\":[\"msg-ambiguous-1\"]}",
    ));
    let messages = vec![message(
        "chat-1",
        "msg-ambiguous-1",
        "MORROW_MESSAGE_CANARY meet Friday afternoon?",
        true,
    )?];
    let mut config = config(550)?;
    config.source_excerpts = SourceExcerptPolicy::Hide;
    let recorder = CollectingRecorder::default();

    let report = DetectionPipeline::new(&provider).detect_with_trace(&messages, &config, &recorder);

    let quiet = only_quiet(&report.outcomes)?;
    assert_eq!(quiet.reason, "confidence_below_threshold");
    let records = recorder.records()?;
    assert_trace_sequence(&records, &threshold_rejection_expectations());
    assert_trace_excludes(
        &records,
        &["MORROW_PROVIDER_TITLE_CANARY_FULL", "MORROW_MESSAGE_CANARY"],
    )?;
    Ok(())
}
