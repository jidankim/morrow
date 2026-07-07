use morrow_detection::{DetectionOutcome, DetectionPipeline};
use morrow_storage::CandidateKind;

use super::support::InspectingProvider;
use super::TestResult;
use crate::support::{config, message};

#[test]
fn selected_context_evidence_ids_are_allowed() -> TestResult {
    let provider = InspectingProvider::new(
        r#"{"kind":"calendar_event","title":"Budget sync",
         "confidence_millis":880,
         "normalized_time":"2026-06-26T10:00:00[Asia/Seoul]",
         "anchor_evidence_id":"evidence://selected/1",
         "evidence_ids":["evidence://selected/0","evidence://selected/1"]}"#,
    );
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![
        message(
            "chat-1",
            "msg-context-companion-001",
            "The budget sync is the planning topic for this week.",
            false,
        )?,
        message(
            "chat-1",
            "msg-context-anchor-001",
            "Friday morning still good for the sync?",
            false,
        )?,
    ];
    let config = config(550)?;

    let report = pipeline.detect(&messages, &config);

    assert_eq!(
        provider.request_guids(),
        vec![vec![
            String::from("msg-context-companion-001"),
            String::from("msg-context-anchor-001"),
        ]]
    );
    assert_eq!(report.candidates().count(), 1);
    match report.outcomes.as_slice() {
        [DetectionOutcome::QuietLog(quiet), DetectionOutcome::Candidate(candidate)] => {
            assert_eq!(quiet.anchor_message_guid, "msg-context-companion-001");
            assert_eq!(quiet.reason, "deterministic_stop:no_scheduling_signal");
            assert_eq!(candidate.kind, CandidateKind::CalendarEvent);
            assert_eq!(candidate.title, "Budget sync");
            assert_eq!(candidate.normalized_time, "2026-06-26T10:00:00[Asia/Seoul]");
            assert_eq!(candidate.anchor_message_guid, "msg-context-anchor-001");
            assert_eq!(
                candidate.evidence_excerpt,
                "Friday morning still good for the sync?"
            );
        }
        outcomes => {
            return Err(format!(
                "expected quiet companion then provider candidate, got {outcomes:?}"
            )
            .into());
        }
    }
    Ok(())
}

#[test]
fn selected_context_anchor_drift_is_rejected() -> TestResult {
    let provider = InspectingProvider::new(
        r#"{"kind":"calendar_event","title":"Budget sync",
         "confidence_millis":880,
         "normalized_time":"2026-06-26T10:00:00[Asia/Seoul]",
         "anchor_evidence_id":"evidence://selected/0",
         "evidence_ids":["evidence://selected/0","evidence://selected/1"]}"#,
    );
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![
        message(
            "chat-1",
            "msg-context-companion-drift-001",
            "The budget sync is the planning topic for this week.",
            false,
        )?,
        message(
            "chat-1",
            "msg-context-anchor-drift-001",
            "Friday morning still good for the sync?",
            false,
        )?,
    ];
    let config = config(550)?;

    let report = pipeline.detect(&messages, &config);

    assert_eq!(
        provider.request_guids(),
        vec![vec![
            String::from("msg-context-companion-drift-001"),
            String::from("msg-context-anchor-drift-001"),
        ]]
    );
    assert_eq!(report.candidates().count(), 0);
    match report.outcomes.as_slice() {
        [DetectionOutcome::QuietLog(companion), DetectionOutcome::QuietLog(anchor)] => {
            assert_eq!(
                companion.anchor_message_guid,
                "msg-context-companion-drift-001"
            );
            assert_eq!(companion.reason, "deterministic_stop:no_scheduling_signal");
            assert_eq!(anchor.anchor_message_guid, "msg-context-anchor-drift-001");
            assert_eq!(anchor.reason, "provider_anchor_not_current_message");
        }
        outcomes => {
            return Err(format!(
                "expected quiet companion then provider anchor rejection, got {outcomes:?}"
            )
            .into());
        }
    }
    Ok(())
}

#[test]
fn unselected_context_guid_is_rejected() -> TestResult {
    let provider = InspectingProvider::new(
        r#"{"kind":"calendar_event","title":"Budget sync",
         "confidence_millis":880,
         "normalized_time":"2026-06-26T10:00:00[Asia/Seoul]",
         "anchor_message_guid":"msg-context-anchor-002",
         "evidence_message_guids":["msg-context-companion-002","msg-context-anchor-002","msg-hallucinated-context"]}"#,
    );
    let pipeline = DetectionPipeline::new(&provider);
    let messages = vec![
        message(
            "chat-1",
            "msg-context-companion-002",
            "The budget sync is the planning topic for this week.",
            false,
        )?,
        message(
            "chat-1",
            "msg-context-anchor-002",
            "Friday morning still good for the sync?",
            false,
        )?,
    ];
    let config = config(550)?;

    let report = pipeline.detect(&messages, &config);

    assert_eq!(
        provider.request_guids(),
        vec![vec![
            String::from("msg-context-companion-002"),
            String::from("msg-context-anchor-002"),
        ]]
    );
    assert_eq!(report.candidates().count(), 0);
    match report.outcomes.as_slice() {
        [DetectionOutcome::QuietLog(companion), DetectionOutcome::QuietLog(anchor)] => {
            assert_eq!(companion.anchor_message_guid, "msg-context-companion-002");
            assert_eq!(companion.reason, "deterministic_stop:no_scheduling_signal");
            assert_eq!(anchor.anchor_message_guid, "msg-context-anchor-002");
            assert_eq!(anchor.reason, "provider_hallucinated_evidence");
        }
        outcomes => {
            return Err(format!(
                "expected quiet companion then provider rejection, got {outcomes:?}"
            )
            .into());
        }
    }
    Ok(())
}

#[test]
fn out_of_prompt_selected_evidence_id_is_rejected() -> TestResult {
    let provider = InspectingProvider::new(
        r#"{"kind":"calendar_event","title":"Hidden sync",
         "confidence_millis":880,
         "normalized_time":"2026-06-26T10:00:00[Asia/Seoul]",
         "anchor_evidence_id":"evidence://selected/20",
         "evidence_ids":["evidence://selected/20"]}"#,
    );
    let pipeline = DetectionPipeline::new(&provider);
    let mut messages = (0..20)
        .map(|index| {
            message(
                "chat-1",
                &format!("msg-prompt-boundary-context-{index:02}"),
                "Context only, not a scheduling request.",
                false,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    messages.push(message(
        "chat-1",
        "msg-prompt-boundary-anchor-20",
        "Friday morning still good for the sync?",
        false,
    )?);
    let config = config(550)?;

    let report = pipeline.detect(&messages, &config);

    let request_guids = provider.request_guids();
    assert_eq!(request_guids.len(), 1);
    let request = request_guids
        .first()
        .ok_or("expected provider to receive one selected evidence request")?;
    assert_eq!(request.len(), 21);
    assert_eq!(report.candidates().count(), 0);
    match report.outcomes.last() {
        Some(DetectionOutcome::QuietLog(quiet)) => {
            assert_eq!(quiet.anchor_message_guid, "msg-prompt-boundary-anchor-20");
            assert_eq!(quiet.reason, "provider_hallucinated_evidence");
        }
        outcome => {
            return Err(format!(
                "expected out-of-prompt evidence rejection for final message, got {outcome:?}"
            )
            .into());
        }
    }
    Ok(())
}
