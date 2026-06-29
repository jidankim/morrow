use std::error::Error;

use morrow_detection::{DetectionPipeline, SourceExcerptPolicy};

use crate::support::{config, message, only_candidate, only_quiet, FakeProvider};

use super::expectations::{parser_candidate_expectations, parser_stop_expectations};
use super::helpers::{
    assert_title_hash_only, assert_trace_excludes, assert_trace_sequence, CollectingRecorder,
};

#[test]
fn trace_records_parser_stop_without_provider_call() -> Result<(), Box<dyn Error>> {
    let provider = FakeProvider::new(None);
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-stop-1",
        "MORROW_MESSAGE_CANARY that movie was funny.",
        false,
    )?];
    let mut config = config(550)?;
    config.source_excerpts = SourceExcerptPolicy::Hide;
    let recorder = CollectingRecorder::default();

    let report = pipeline.detect_with_trace(&messages, &config, &recorder);

    let quiet = only_quiet(&report.outcomes)?;
    assert_eq!(quiet.reason, "deterministic_stop:no_scheduling_signal");
    assert_eq!(provider.calls(), 0);
    let records = recorder.records()?;
    assert_trace_sequence(
        &records,
        &parser_stop_expectations("deterministic_stop:no_scheduling_signal"),
    );
    assert_trace_excludes(&records, &["MORROW_MESSAGE_CANARY"])?;
    Ok(())
}

#[test]
fn trace_records_parser_candidate_without_provider_call() -> Result<(), Box<dyn Error>> {
    let provider = FakeProvider::new(None);
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-candidate-1",
        "MORROW_MESSAGE_CANARY meet 2026-06-27 14:00 for lunch.",
        false,
    )?];
    let mut config = config(550)?;
    config.source_excerpts = SourceExcerptPolicy::Hide;
    let recorder = CollectingRecorder::default();

    let report = pipeline.detect_with_trace(&messages, &config, &recorder);

    let candidate = only_candidate(&report.outcomes)?;
    assert_eq!(
        candidate.evidence_excerpt,
        "Source excerpt hidden by settings."
    );
    assert_eq!(provider.calls(), 0);
    let records = recorder.records()?;
    assert_trace_sequence(&records, &parser_candidate_expectations());
    assert_title_hash_only(&records, 1);
    assert_trace_excludes(&records, &["MORROW_MESSAGE_CANARY"])?;
    Ok(())
}
