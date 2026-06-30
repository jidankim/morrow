mod decision {
    include!("../../src/native_bridge/scan/proposal_replay/decision.rs");
}

use morrow_storage::{CandidateId, CandidateKind, ExternalObjectMapping, ExternalSource};

use decision::{
    candidate_replay_decision, creation_failure_decision, mapping_finalization_decision,
    CandidateReplayDecision, ProposalReplayDelta, ReplayMapping, ReplayStoreDecision,
};

#[test]
fn proposal_replay_decision_marks_recovery_pending_on_finalize_failure() -> Result<(), String> {
    // Given
    let mapping = calendar_mapping()?;

    // When
    let existing = candidate_replay_decision(CandidateKind::CalendarEvent, Some(mapping.clone()));
    let calendar_create = candidate_replay_decision(CandidateKind::CalendarEvent, None);
    let legacy_create = candidate_replay_decision(CandidateKind::TaskReminder, None);
    let created = mapping_finalization_decision(true, true);
    let not_created = mapping_finalization_decision(false, true);
    let recovery = mapping_finalization_decision(true, false);
    let failed = creation_failure_decision(Some("Calendar source unavailable"));
    let replay_mapping = ReplayMapping {
        mapping: mapping.clone(),
        created_external: true,
    };

    // Then
    assert_mapping_present(existing, &mapping)?;
    assert!(matches!(
        calendar_create,
        CandidateReplayDecision::NeedsCalendarCreate
    ));
    assert!(matches!(
        legacy_create,
        CandidateReplayDecision::NeedsLegacyCreate
    ));
    assert_eq!(
        created,
        ReplayStoreDecision::TransitionVisible {
            summary_delta: ProposalReplayDelta {
                created: 1,
                failed: 0,
            },
        }
    );
    assert_eq!(
        not_created,
        ReplayStoreDecision::TransitionVisible {
            summary_delta: ProposalReplayDelta {
                created: 0,
                failed: 0,
            },
        }
    );
    assert_eq!(
        recovery,
        ReplayStoreDecision::MarkRecoveryPending {
            summary_delta: ProposalReplayDelta {
                created: 0,
                failed: 1,
            },
        }
    );
    assert_eq!(
        failed,
        ReplayStoreDecision::MarkFailed {
            reason: "external_proposal_creation_failed: Calendar source unavailable".to_owned(),
            summary_delta: ProposalReplayDelta {
                created: 0,
                failed: 1,
            },
        }
    );
    assert!(replay_mapping.created_external);
    assert_eq!(
        replay_mapping.mapping.external_object_id,
        "event-1".to_owned()
    );
    Ok(())
}

fn calendar_mapping() -> Result<ExternalObjectMapping, String> {
    Ok(ExternalObjectMapping {
        candidate_id: CandidateId::from_storage("morrow_0000000000000001")
            .map_err(|error| error.to_string())?,
        source: ExternalSource::Calendar,
        external_object_id: "event-1".to_owned(),
        external_source_id: "source-1".to_owned(),
        mapped_at: 1_782_352_400,
    })
}

fn assert_mapping_present(
    decision: CandidateReplayDecision,
    expected: &ExternalObjectMapping,
) -> Result<(), String> {
    match decision {
        CandidateReplayDecision::MappingPresent(actual) => {
            assert_eq!(actual.candidate_id.as_str(), expected.candidate_id.as_str());
            assert_eq!(actual.source, expected.source);
            assert_eq!(actual.external_object_id, expected.external_object_id);
            assert_eq!(actual.external_source_id, expected.external_source_id);
            assert_eq!(actual.mapped_at, expected.mapped_at);
            Ok(())
        }
        CandidateReplayDecision::NeedsCalendarCreate => {
            Err("existing mapping requested calendar create".to_owned())
        }
        CandidateReplayDecision::NeedsLegacyCreate => {
            Err("existing mapping requested legacy create".to_owned())
        }
    }
}
