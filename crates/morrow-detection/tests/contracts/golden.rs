use std::error::Error;

use morrow_detection::DetectionPipeline;

use crate::support::{config_from_fixture, FakeProvider, Fixture, MessageFixture};

#[test]
fn golden_conversation_fixtures_match_pipeline_contract() -> Result<(), Box<dyn Error>> {
    // Given
    let fixture: Fixture =
        serde_json::from_str(include_str!("../../fixtures/golden_conversations.json"))?;
    let config = config_from_fixture(&fixture)?;

    for scenario in fixture.scenarios {
        let provider = FakeProvider::from_owned(scenario.provider_response);
        let messages = scenario
            .messages
            .iter()
            .map(MessageFixture::to_message)
            .collect::<Result<Vec<_>, _>>()?;

        // When
        let report = DetectionPipeline::new(&provider).detect(&messages, &config);

        // Then
        assert_eq!(
            scenario.expected_provider_calls,
            provider.calls(),
            "{}",
            scenario.name
        );
        assert_eq!(
            scenario.expected_candidate_count,
            report.candidates().count(),
            "{}",
            scenario.name
        );
        assert_eq!(
            scenario.expected_quiet_count,
            report.quiet_logs().count(),
            "{}",
            scenario.name
        );
    }

    Ok(())
}
