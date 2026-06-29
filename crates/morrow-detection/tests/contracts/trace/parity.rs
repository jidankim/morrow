use std::error::Error;

use morrow_detection::DetectionPipeline;
use morrow_diagnostics::NoopTraceRecorder;

use crate::support::{config, message, only_candidate, FakeProvider};

#[test]
fn detect_with_noop_trace_matches_default_contract() -> Result<(), Box<dyn Error>> {
    let response = "{\"kind\":\"calendar_event\",\"title\":\"Meet Friday afternoon\",\
         \"confidence_millis\":720,\
         \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
         \"anchor_message_guid\":\"msg-ambiguous-1\",\
         \"evidence_message_guids\":[\"msg-ambiguous-1\"]}";
    let default_provider = FakeProvider::new(Some(response));
    let traced_provider = FakeProvider::new(Some(response));
    let messages = vec![message(
        "chat-1",
        "msg-ambiguous-1",
        "Can we meet Friday afternoon?",
        true,
    )?];
    let config = config(550)?;
    let recorder = NoopTraceRecorder;

    let default_report = DetectionPipeline::new(&default_provider).detect(&messages, &config);
    let traced_report =
        DetectionPipeline::new(&traced_provider).detect_with_trace(&messages, &config, &recorder);

    assert_eq!(default_provider.calls(), traced_provider.calls());
    assert_eq!(
        default_report.candidates().count(),
        traced_report.candidates().count()
    );
    assert_eq!(
        default_report.quiet_logs().count(),
        traced_report.quiet_logs().count()
    );
    let default_candidate = only_candidate(&default_report.outcomes)?;
    let traced_candidate = only_candidate(&traced_report.outcomes)?;
    assert_eq!(default_candidate.title, traced_candidate.title);
    assert_eq!(
        default_candidate.normalized_time,
        traced_candidate.normalized_time
    );
    assert_eq!(
        default_candidate.confidence_millis,
        traced_candidate.confidence_millis
    );
    Ok(())
}
