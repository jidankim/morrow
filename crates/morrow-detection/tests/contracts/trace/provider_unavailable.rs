use std::error::Error;

use morrow_detection::{DetectionPipeline, SourceExcerptPolicy};
use morrow_diagnostics::NoopTraceRecorder;

use crate::support::{config, message, only_quiet};

use super::expectations::provider_unavailable_expectations;
use super::helpers::{
    assert_trace_excludes, assert_trace_sequence, CollectingRecorder, UnavailableProvider,
};

#[test]
fn trace_records_provider_unavailable_matches_default_paths() -> Result<(), Box<dyn Error>> {
    let traced_provider = UnavailableProvider::default();
    let default_provider = UnavailableProvider::default();
    let noop_provider = UnavailableProvider::default();
    let messages = vec![message(
        "chat-1",
        "msg-provider-down",
        "Can we meet Friday afternoon?",
        true,
    )?];
    let mut config = config(550)?;
    config.source_excerpts = SourceExcerptPolicy::Hide;
    let recorder = CollectingRecorder::default();
    let noop = NoopTraceRecorder;

    let traced_report =
        DetectionPipeline::new(&traced_provider).detect_with_trace(&messages, &config, &recorder);
    let default_report = DetectionPipeline::new(&default_provider).detect(&messages, &config);
    let noop_report =
        DetectionPipeline::new(&noop_provider).detect_with_trace(&messages, &config, &noop);

    assert_eq!(
        only_quiet(&traced_report.outcomes)?.reason,
        "provider_unavailable"
    );
    assert_eq!(
        only_quiet(&default_report.outcomes)?.reason,
        "provider_unavailable"
    );
    assert_eq!(
        only_quiet(&noop_report.outcomes)?.reason,
        "provider_unavailable"
    );
    assert_eq!(traced_provider.calls(), 1);
    assert_eq!(default_provider.calls(), traced_provider.calls());
    assert_eq!(noop_provider.calls(), traced_provider.calls());
    assert_eq!(
        traced_report.quiet_logs().count(),
        default_report.quiet_logs().count()
    );
    assert_eq!(
        traced_report.quiet_logs().count(),
        noop_report.quiet_logs().count()
    );
    assert_eq!(
        traced_report.candidates().count(),
        default_report.candidates().count()
    );
    assert_eq!(
        traced_report.candidates().count(),
        noop_report.candidates().count()
    );
    let records = recorder.records()?;
    assert_trace_sequence(&records, &provider_unavailable_expectations());
    assert_trace_excludes(&records, &["Can we meet Friday afternoon?"])?;
    Ok(())
}
