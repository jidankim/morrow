use std::error::Error;

use morrow_detection::{DetectionPipeline, SourceExcerptPolicy};
use morrow_diagnostics::TraceRecord;
use serde::Serialize;

use crate::support::{
    config_from_fixture, CollectingRecorder, FakeProvider, Fixture, MessageFixture, Scenario,
};

#[test]
fn golden_conversation_fixtures_match_pipeline_contract() -> Result<(), Box<dyn Error>> {
    // Given
    let fixture: Fixture =
        serde_json::from_str(include_str!("../../fixtures/golden_conversations.json"))?;

    // When / Then
    assert_fixture_matches(&fixture).map_err(Into::into)
}

#[test]
fn golden_count_preserving_route_regression_fails() -> Result<(), Box<dyn Error>> {
    // Given
    let mut fixture: Fixture =
        serde_json::from_str(include_str!("../../fixtures/golden_conversations.json"))?;
    let scenario = fixture
        .scenarios
        .iter_mut()
        .find(|scenario| scenario.name == "ambiguous_provider_routed_candidate")
        .ok_or("missing ambiguous_provider_routed_candidate scenario")?;
    let first = scenario
        .expected_trace
        .first_mut()
        .ok_or("missing first trace expectation")?;
    first.decision = Some(morrow_diagnostics::TraceDecision::Candidate);
    first.reason_code = Some("parser_candidate".to_owned());

    // When
    let result = assert_fixture_matches(&fixture);

    // Then
    let message = result.err().ok_or("expected bad fixture to be rejected")?;
    assert!(
        message.contains("ambiguous_provider_routed_candidate")
            && message.contains("step 0")
            && message.contains("decision")
            && message.contains("candidate")
            && message.contains("provider_route"),
        "{message}"
    );
    Ok(())
}

fn assert_fixture_matches(fixture: &Fixture) -> Result<(), String> {
    let mut config = config_from_fixture(fixture).map_err(|err| err.to_string())?;
    config.source_excerpts = SourceExcerptPolicy::Hide;

    for scenario in &fixture.scenarios {
        let provider = FakeProvider::from_owned(scenario.provider_response.clone());
        let messages = scenario
            .messages
            .iter()
            .map(MessageFixture::to_message)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| scenario_error(scenario, err))?;
        let recorder = CollectingRecorder::default();

        // When
        let report =
            DetectionPipeline::new(&provider).detect_with_trace(&messages, &config, &recorder);

        // Then
        CountCheck::new(scenario, "provider calls")
            .compare(scenario.expected_provider_calls, provider.calls())?;
        CountCheck::new(scenario, "candidate count").compare(
            scenario.expected_candidate_count,
            report.candidates().count(),
        )?;
        CountCheck::new(scenario, "quiet log count")
            .compare(scenario.expected_quiet_count, report.quiet_logs().count())?;
        let records = recorder
            .records()
            .map_err(|err| scenario_error(scenario, err))?;
        assert_ordered_trace(scenario, &records)?;
        assert_trace_is_sanitized(scenario, &records)?;
    }

    Ok(())
}

fn assert_ordered_trace(scenario: &Scenario, records: &[TraceRecord]) -> Result<(), String> {
    if scenario.expected_trace.len() != records.len() {
        return Err(format!(
            "{} trace length mismatch: expected {}, actual {}",
            scenario.name,
            scenario.expected_trace.len(),
            records.len()
        ));
    }
    for (index, (expected, record)) in scenario.expected_trace.iter().zip(records).enumerate() {
        FieldContext::new(&scenario.name, index, "component")
            .check(&expected.component, &record.span.component)?;
        FieldContext::new(&scenario.name, index, "operation")
            .check(&expected.operation, &record.span.operation)?;
        FieldContext::new(&scenario.name, index, "decision")
            .check(&expected.decision, &record.span.decision)?;
        FieldContext::new(&scenario.name, index, "outcome")
            .check(&expected.outcome, &record.span.outcome)?;
        FieldContext::new(&scenario.name, index, "reason_code")
            .check(&expected.reason_code, &record.span.reason_code)?;
        FieldContext::new(&scenario.name, index, "privacy_tier")
            .check(&expected.privacy_tier, &record.span.privacy_tier)?;
    }
    Ok(())
}

fn assert_trace_is_sanitized(scenario: &Scenario, records: &[TraceRecord]) -> Result<(), String> {
    let serialized = serde_json::to_string(records).map_err(|err| scenario_error(scenario, err))?;
    for message in &scenario.messages {
        if serialized.contains(&message.excerpt) {
            return Err(format!("{} trace leaked message excerpt", scenario.name));
        }
    }
    if let Some(provider_response) = &scenario.provider_response {
        if serialized.contains(provider_response) {
            return Err(format!("{} trace leaked provider response", scenario.name));
        }
    }
    for forbidden in [
        "raw_text",
        "response",
        "raw_json",
        "provider_json",
        "full_message",
        "raw_title",
        "title_text",
        "full_title",
        "unredacted_title",
    ] {
        if serialized.contains(forbidden) {
            return Err(format!("{} trace leaked {forbidden}", scenario.name));
        }
    }
    Ok(())
}

struct CountCheck<'a> {
    scenario: &'a Scenario,
    label: &'static str,
}

impl<'a> CountCheck<'a> {
    const fn new(scenario: &'a Scenario, label: &'static str) -> Self {
        Self { scenario, label }
    }

    fn compare(&self, expected: usize, actual: usize) -> Result<(), String> {
        if expected == actual {
            return Ok(());
        }
        Err(format!(
            "{} {} mismatch: expected {expected}, actual {actual}",
            self.scenario.name, self.label
        ))
    }
}

struct FieldContext<'a> {
    scenario: &'a str,
    step: usize,
    field: &'static str,
}

impl<'a> FieldContext<'a> {
    const fn new(scenario: &'a str, step: usize, field: &'static str) -> Self {
        Self {
            scenario,
            step,
            field,
        }
    }

    fn check<T>(&self, expected: &T, actual: &T) -> Result<(), String>
    where
        T: PartialEq + Serialize,
    {
        if expected == actual {
            return Ok(());
        }
        Err(format!(
            "{} step {} field {} mismatch: expected {}, actual {}",
            self.scenario,
            self.step,
            self.field,
            json_value(expected),
            json_value(actual)
        ))
    }
}

fn json_value<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "<unserializable>".to_owned())
}

fn scenario_error<E: std::fmt::Display>(scenario: &Scenario, error: E) -> String {
    format!("{}: {error}", scenario.name)
}
