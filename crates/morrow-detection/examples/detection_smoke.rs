use std::cell::Cell;
use std::error::Error;

use morrow_detection::{
    AiProvider, ConfidenceThreshold, DetectionConfig, DetectionOutcome, DetectionPipeline,
    ProviderError, ProviderIdentity, ProviderRequest, ProviderResponse, ReferenceTime,
    SourceExcerptPolicy,
};
use morrow_messages::{ChatGuid, MessageEvidence, MessageGuid, MessageTimestamp};
use serde::Deserialize;

struct SmokeProvider {
    response: Option<String>,
    calls: Cell<usize>,
}

impl SmokeProvider {
    const fn new(response: Option<String>) -> Self {
        Self {
            response,
            calls: Cell::new(0),
        }
    }

    const fn calls(&self) -> usize {
        self.calls.get()
    }
}

impl AiProvider for SmokeProvider {
    fn extract(&self, request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        self.calls.set(self.calls.get() + 1);
        if request.evidence().is_empty() {
            return Err(ProviderError::Unavailable {
                reason: "empty evidence".to_owned(),
            });
        }
        let response = self.response.as_deref().unwrap_or(
            "{\"kind\":\"calendar_event\",\"title\":\"Fallback\",\
             \"confidence_millis\":720,\
             \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
             \"anchor_message_guid\":\"msg-ambiguous-1\",\
             \"evidence_message_guids\":[\"msg-ambiguous-1\"]}",
        );
        Ok(ProviderResponse::new(response))
    }
}

#[derive(Deserialize)]
struct Fixture {
    reference_time: String,
    timezone: String,
    threshold_millis: i64,
    scenarios: Vec<Scenario>,
}

#[derive(Deserialize)]
struct Scenario {
    name: String,
    messages: Vec<MessageFixture>,
    provider_response: Option<String>,
    expected_candidate_count: usize,
    expected_quiet_count: usize,
    expected_provider_calls: usize,
}

#[derive(Deserialize)]
struct MessageFixture {
    chat_guid: String,
    message_guid: String,
    timestamp: i64,
    participant_count: u16,
    tapback_signal: bool,
    excerpt: String,
    evidence_pointer: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let scenario = std::env::args().nth(2).unwrap_or_default();
    if scenario != "golden-fixtures" {
        return Err("usage: detection_smoke --scenario golden-fixtures".into());
    }

    let fixture: Fixture =
        serde_json::from_str(include_str!("../fixtures/golden_conversations.json"))?;
    let config = DetectionConfig {
        reference: ReferenceTime::parse(&fixture.reference_time, &fixture.timezone)?,
        threshold: ConfidenceThreshold::new(fixture.threshold_millis)?,
        provider: ProviderIdentity::new("fake-provider", "offline-contract", "prompt-v1")?,
        source_excerpts: SourceExcerptPolicy::Include,
    };

    let mut candidates_created = 0usize;
    let mut quiet_logs_created = 0usize;
    let mut provider_calls = 0usize;
    let mut hallucinated_rejected = false;
    let mut parser_conflict_quiet_logged = false;

    for scenario in fixture.scenarios {
        let provider = SmokeProvider::new(scenario.provider_response);
        let messages = scenario
            .messages
            .iter()
            .map(MessageFixture::to_message)
            .collect::<Result<Vec<_>, _>>()?;
        let report = DetectionPipeline::new(&provider).detect(&messages, &config);
        let candidate_count = report.candidates().count();
        let quiet_count = report.quiet_logs().count();
        if candidate_count != scenario.expected_candidate_count
            || quiet_count != scenario.expected_quiet_count
            || provider.calls() != scenario.expected_provider_calls
        {
            return Err(format!("scenario failed: {}", scenario.name).into());
        }

        candidates_created += candidate_count;
        quiet_logs_created += quiet_count;
        provider_calls += provider.calls();
        if scenario.name == "hallucinated_evidence" {
            hallucinated_rejected = report.quiet_logs().any(|quiet| {
                quiet.reason == "provider_hallucinated_evidence"
                    && !quiet.excerpt.contains("Made up")
            });
        }
        if scenario.name == "parser_provider_conflict" {
            parser_conflict_quiet_logged = report
                .quiet_logs()
                .any(|quiet| quiet.reason == "parser_provider_time_conflict");
        }
        for outcome in report.outcomes {
            match outcome {
                DetectionOutcome::Candidate(candidate) => {
                    if candidate.evidence_excerpt.lines().count() > 3 {
                        return Err("candidate stored too much evidence text".into());
                    }
                }
                DetectionOutcome::QuietLog(quiet) => {
                    if quiet.excerpt.lines().count() > 3 {
                        return Err("quiet log stored too much evidence text".into());
                    }
                }
            }
        }
    }

    if !hallucinated_rejected || !parser_conflict_quiet_logged {
        return Err("required rejection assertions did not fire".into());
    }

    println!(
        "candidates_created={candidates_created} quiet_logs_created={quiet_logs_created} \
         provider_calls={provider_calls} hallucinated_rejected={hallucinated_rejected} \
         parser_conflict_quiet_logged={parser_conflict_quiet_logged}"
    );
    println!("PASS detection_smoke");
    Ok(())
}

impl MessageFixture {
    fn to_message(&self) -> Result<MessageEvidence, Box<dyn Error>> {
        Ok(MessageEvidence {
            chat_guid: ChatGuid::parse(&self.chat_guid)?,
            message_guid: MessageGuid::parse(&self.message_guid)?,
            timestamp: MessageTimestamp::new(self.timestamp)?,
            participant_count: self.participant_count,
            tapback_signal: self.tapback_signal,
            excerpt: self.excerpt.clone(),
            evidence_pointer: self.evidence_pointer.clone(),
        })
    }
}
