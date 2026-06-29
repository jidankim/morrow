use std::io::Error;
use std::path::PathBuf;

use morrow_reconcile::{CandidateLifecycle, LifecycleReason};
use morrow_storage::{
    CandidateDraft, CandidateId, CandidateKind, CandidateState, ExternalObjectMapping,
    ExternalSource, Store,
};

/// Creates a candidate in the requested setup state.
pub fn prepare_candidate(
    store: &Store,
    kind: CandidateKind,
    suffix: &str,
    state: CandidateState,
) -> Result<CandidateId, Box<dyn std::error::Error>> {
    let candidate_id = store.create_candidate(draft(kind, suffix))?;
    match state {
        CandidateState::Queued => {}
        CandidateState::CreatingExternal => {
            store.transition_candidate(
                &candidate_id,
                CandidateState::CreatingExternal,
                "setup_creating",
                2,
            )?;
        }
        CandidateState::Visible | CandidateState::Approved => {
            store.transition_candidate(
                &candidate_id,
                CandidateState::CreatingExternal,
                "setup_creating",
                2,
            )?;
            store.transition_candidate(
                &candidate_id,
                CandidateState::Visible,
                "setup_visible",
                3,
            )?;
            if state == CandidateState::Approved {
                store.transition_candidate(
                    &candidate_id,
                    CandidateState::Approved,
                    "setup_approved",
                    4,
                )?;
            }
        }
        CandidateState::Completed
        | CandidateState::Rejected
        | CandidateState::Expired
        | CandidateState::Suppressed
        | CandidateState::Unknown
        | CandidateState::Failed => return Err(Box::new(Error::other("unsupported setup state"))),
    }
    Ok(candidate_id)
}

/// Builds a candidate lifecycle fixture.
pub fn lifecycle(
    candidate_id: &CandidateId,
    kind: CandidateKind,
    state: CandidateState,
    suffix: &str,
    observed_at: i64,
) -> CandidateLifecycle {
    CandidateLifecycle {
        candidate_id: candidate_id.clone(),
        kind,
        state,
        mapping: Some(mapping(
            candidate_id,
            source_for_kind(kind),
            &format!("proposed-{suffix}"),
            1,
        )),
        observed_at,
    }
}

/// Counts audit entries with the requested lifecycle reason.
pub fn count_reason(
    store: &Store,
    candidate_id: &CandidateId,
    reason: LifecycleReason,
) -> Result<usize, Box<dyn std::error::Error>> {
    Ok(store
        .audit_entries(candidate_id)?
        .iter()
        .filter(|entry| entry.reason == reason.as_str())
        .count())
}

/// Builds an external object mapping fixture.
pub fn mapping(
    candidate_id: &CandidateId,
    source: ExternalSource,
    external_object_id: &str,
    mapped_at: i64,
) -> ExternalObjectMapping {
    ExternalObjectMapping {
        candidate_id: candidate_id.clone(),
        source,
        external_object_id: external_object_id.to_owned(),
        external_source_id: "source-main".to_owned(),
        mapped_at,
    }
}

/// Returns the smoke-suite database path.
pub fn db_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "morrow-reconcile-smoke-{}.sqlite3",
        std::process::id()
    ))
}

fn draft(kind: CandidateKind, suffix: &str) -> CandidateDraft {
    CandidateDraft {
        kind,
        chat_guid: format!("chat-{suffix}"),
        anchor_message_guid: format!("message-{suffix}"),
        title: format!("Candidate {suffix}"),
        confidence_millis: 850,
        normalized_time: format!("2026-07-01T10:{:02}:00Z", suffix.len()),
        evidence_excerpt: "See you then".to_owned(),
        observed_at: 1,
    }
}

const fn source_for_kind(kind: CandidateKind) -> ExternalSource {
    match kind {
        CandidateKind::CalendarEvent
        | CandidateKind::EventUpdate
        | CandidateKind::EventReschedule
        | CandidateKind::EventCancellation => ExternalSource::Calendar,
        CandidateKind::TaskReminder
        | CandidateKind::ReminderUpdate
        | CandidateKind::ReminderReschedule
        | CandidateKind::ReminderCancellation => ExternalSource::Reminders,
    }
}
