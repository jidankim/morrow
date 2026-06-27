use std::error::Error;
use std::path::Path;
use std::path::PathBuf;

use morrow_reconcile::{
    apply_reconciliation, reconcile_candidate, CandidateLifecycle, ExternalItemObservation,
    LifecycleAction, LifecycleReason,
};
use morrow_storage::{
    CandidateDraft, CandidateKind, CandidateState, ExternalObjectMapping, ExternalSource,
    ReplayStream, Store,
};

#[derive(Debug, Clone)]
pub(crate) struct ReplaySummary {
    pub(crate) recovered_reason_count: usize,
    pub(crate) replay_create_actions: usize,
    pub(crate) replay_candidates: usize,
}

pub(crate) fn run() -> Result<ReplaySummary, Box<dyn Error>> {
    let db_path = db_path();
    let result = run_with_db(&db_path);
    let cleanup = std::fs::remove_file(&db_path);
    match (result, cleanup) {
        (Ok(summary), Ok(())) => Ok(summary),
        (Ok(_), Err(error)) => Err(error.into()),
        (Err(error), _) => Err(error),
    }
}

fn run_with_db(db_path: &Path) -> Result<ReplaySummary, Box<dyn Error>> {
    let store = Store::open(db_path)?;
    let candidate_id = store.create_candidate(draft())?;
    store.transition_candidate(
        &candidate_id,
        CandidateState::CreatingExternal,
        "setup_creating",
        2,
    )?;
    let mapping = ExternalObjectMapping {
        candidate_id: candidate_id.clone(),
        source: ExternalSource::Calendar,
        external_object_id: "eventkit://partial-write".to_owned(),
        external_source_id: "source-primary".to_owned(),
        mapped_at: 3,
    };
    store.upsert_external_mapping(mapping.clone())?;

    let recovered = reconcile_candidate(
        &CandidateLifecycle {
            candidate_id: candidate_id.clone(),
            kind: CandidateKind::CalendarEvent,
            state: CandidateState::CreatingExternal,
            mapping: Some(mapping.clone()),
            observed_at: 4,
        },
        &ExternalItemObservation::Pending,
    )?;
    if recovered
        .actions
        .iter()
        .any(|action| matches!(action, LifecycleAction::CreateExternalProposal))
    {
        return Err(std::io::Error::other("duplicate proposal action emitted").into());
    }
    apply_reconciliation(&store, &recovered)?;

    let replay = reconcile_candidate(
        &CandidateLifecycle {
            candidate_id: candidate_id.clone(),
            kind: CandidateKind::CalendarEvent,
            state: CandidateState::Visible,
            mapping: Some(mapping),
            observed_at: 5,
        },
        &ExternalItemObservation::Pending,
    )?;
    let replay_create_actions = replay
        .actions
        .iter()
        .filter(|action| matches!(action, LifecycleAction::CreateExternalProposal))
        .count();
    let recovered_reason_count = store
        .audit_entries(&candidate_id)?
        .iter()
        .filter(|entry| entry.reason == LifecycleReason::PartialWriteRecovered.as_str())
        .count();
    let replay_candidates = store
        .next_replay_candidates(ReplayStream::CalendarProposals, 10)?
        .len();

    Ok(ReplaySummary {
        recovered_reason_count,
        replay_create_actions,
        replay_candidates,
    })
}

fn draft() -> CandidateDraft {
    CandidateDraft {
        kind: CandidateKind::CalendarEvent,
        chat_guid: "chat-partial".to_owned(),
        anchor_message_guid: "message-partial".to_owned(),
        title: "Partial write".to_owned(),
        confidence_millis: 850,
        normalized_time: "2026-07-01T10:00:00Z".to_owned(),
        evidence_excerpt: "See you then".to_owned(),
        observed_at: 1,
    }
}

fn db_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "morrow-task11-native-failure-{}.sqlite3",
        std::process::id()
    ))
}
