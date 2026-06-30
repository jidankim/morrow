use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use morrow_reconcile::{CandidateLifecycle, LifecycleReason};
use morrow_storage::{
    CandidateDraft, CandidateId, CandidateKind, CandidateState, ExternalObjectMapping,
    ExternalSource, Store,
};

static NEXT_DB: AtomicU64 = AtomicU64::new(1);

/// Builds a lifecycle fixture without an external mapping.
pub fn visible_lifecycle(kind: CandidateKind, state: CandidateState) -> CandidateLifecycle {
    CandidateLifecycle {
        candidate_id: CandidateId::derive(kind, "chat", "message", "2026-07-01T10:00:00Z"),
        kind,
        state,
        mapping: None,
        observed_at: 10,
    }
}

/// Creates a visible candidate fixture.
pub fn visible_candidate(
    store: &Store,
    kind: CandidateKind,
    suffix: &str,
) -> Result<CandidateId, TestError> {
    let candidate_id = creating_candidate(store, kind, suffix)?;
    store.transition_candidate(&candidate_id, CandidateState::Visible, "setup_visible", 3)?;
    Ok(candidate_id)
}

/// Creates a candidate fixture in the `CreatingExternal` state.
pub fn creating_candidate(
    store: &Store,
    kind: CandidateKind,
    suffix: &str,
) -> Result<CandidateId, TestError> {
    let candidate_id = store.create_candidate(draft(kind, suffix))?;
    store.transition_candidate(
        &candidate_id,
        CandidateState::CreatingExternal,
        "setup_creating",
        2,
    )?;
    Ok(candidate_id)
}

/// Builds a candidate draft fixture.
pub fn draft(kind: CandidateKind, suffix: &str) -> CandidateDraft {
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

/// Builds a calendar external mapping fixture.
pub fn mapping(candidate_id: &CandidateId, external_object_id: &str) -> ExternalObjectMapping {
    mapping_with_source(candidate_id, ExternalSource::Calendar, external_object_id)
}

/// Builds an external mapping fixture for the requested source.
pub fn mapping_with_source(
    candidate_id: &CandidateId,
    source: ExternalSource,
    external_object_id: &str,
) -> ExternalObjectMapping {
    ExternalObjectMapping {
        candidate_id: candidate_id.clone(),
        source,
        external_object_id: external_object_id.to_owned(),
        external_source_id: "source-main".to_owned(),
        mapped_at: 1,
    }
}

/// Asserts that the candidate audit log contains the requested reason.
///
/// # Panics
///
/// Panics when the expected reason is not present in the audit log.
pub fn assert_audit_reason(
    store: &Store,
    candidate_id: &CandidateId,
    reason: LifecycleReason,
) -> Result<(), TestError> {
    let entries = store.audit_entries(candidate_id)?;
    assert!(entries.iter().any(|entry| entry.reason == reason.as_str()));
    Ok(())
}

/// Returns a unique test database path.
pub fn db_path(name: &str) -> PathBuf {
    let next = NEXT_DB.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "morrow-reconcile-{name}-{}-{next}.sqlite3",
        std::process::id()
    ))
}

/// Error type used by reconcile integration tests.
#[derive(Debug)]
pub enum TestError {
    /// Reconciliation error.
    Reconcile(morrow_reconcile::ReconcileError),
    /// Storage error.
    Storage(morrow_storage::StorageError),
    /// SQLite assertion command failure.
    Command(String),
    /// Expected error was not produced.
    MissingExpectedError,
}

impl From<morrow_reconcile::ReconcileError> for TestError {
    fn from(value: morrow_reconcile::ReconcileError) -> Self {
        Self::Reconcile(value)
    }
}

impl From<morrow_storage::StorageError> for TestError {
    fn from(value: morrow_storage::StorageError) -> Self {
        Self::Storage(value)
    }
}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reconcile(err) => write!(f, "{err}"),
            Self::Storage(err) => write!(f, "{err}"),
            Self::Command(err) => f.write_str(err),
            Self::MissingExpectedError => f.write_str("missing expected error"),
        }
    }
}

impl std::error::Error for TestError {}
