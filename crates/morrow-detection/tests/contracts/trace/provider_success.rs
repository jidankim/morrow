use std::error::Error;

use morrow_detection::{DetectionPipeline, SourceExcerptPolicy};
use morrow_diagnostics::TracePrivacyTier;

use crate::support::{config, message, only_candidate, FakeProvider};

use super::expectations::provider_success_expectations;
use super::helpers::{
    assert_provider_metadata_prefix, assert_title_hash_only, assert_trace_excludes,
    assert_trace_sequence, CollectingRecorder,
};

#[test]
fn trace_records_provider_success_threshold_and_privacy_state() -> Result<(), Box<dyn Error>> {
    let provider = FakeProvider::new(Some(
        "{\"kind\":\"calendar_event\",\"title\":\"MORROW_PROVIDER_TITLE_CANARY_FULL\",\
         \"confidence_millis\":800,\
         \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
         \"anchor_message_guid\":\"msg-ambiguous-1\",\
         \"evidence_message_guids\":[\"msg-ambiguous-1\"]}",
    ));
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-ambiguous-1",
        "MORROW_MESSAGE_CANARY meet Friday afternoon?",
        true,
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
    let records = recorder.records()?;
    assert_trace_sequence(&records, &provider_success_expectations());
    assert_provider_metadata_prefix(&records, 5);
    assert_title_hash_only(&records, 1);
    assert_eq!(
        records
            .iter()
            .filter(|record| record.span.privacy_tier == TracePrivacyTier::LocalPrivate)
            .count(),
        1
    );
    assert_trace_excludes(
        &records,
        &[
            "MORROW_PROVIDER_TITLE_CANARY_FULL",
            "MORROW_MESSAGE_CANARY",
            "chat-1",
            "msg-ambiguous-1",
            "messages://chat-1/msg-ambiguous-1",
            "raw_title",
            "title_text",
            "full_title",
            "unredacted_title",
        ],
    )?;
    Ok(())
}
