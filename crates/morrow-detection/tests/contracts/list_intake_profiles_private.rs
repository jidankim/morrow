use std::error::Error;

use morrow_detection::DetectionPipeline;

use crate::support::{config_with_profile_bare_quantity_lists, message, only_quiet, FakeProvider};

#[test]
fn malformed_private_risk_list_stays_out_of_scheduling_provider() -> Result<(), Box<dyn Error>> {
    // Given
    let provider = FakeProvider::new(None);
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![message(
        "chat-1",
        "msg-list-intake-private-risk-1",
        "2 a@example.com, 3 salmon",
        false,
    )?];
    let config = config_with_profile_bare_quantity_lists(550)?;

    // When
    let report = pipeline.detect(&messages, &config);

    // Then
    assert_eq!(provider.calls(), 0);
    let quiet = only_quiet(&report.outcomes)?;
    assert_eq!(quiet.reason, "deterministic_stop:no_scheduling_signal");
    println!(
        "malformed_private_risk_list_stays_out_of_scheduling_provider provider_calls={} quiet_reason={}",
        provider.calls(),
        quiet.reason
    );
    Ok(())
}
