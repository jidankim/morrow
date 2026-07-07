use morrow_storage::{CandidateDraft, CandidateId, CandidateKind, CandidateState, Store};

pub fn fresh_store(name: &str) -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join(name);
    let store = Store::open(&db_path).expect("open store");
    (dir, store)
}

fn lifecycle_draft(kind: CandidateKind, anchor: &str, normalized_time: &str) -> CandidateDraft {
    CandidateDraft {
        kind,
        chat_guid: "iMessage;+;+15555550100".to_owned(),
        anchor_message_guid: anchor.to_owned(),
        title: "Dentist appointment".to_owned(),
        confidence_millis: 850,
        normalized_time: normalized_time.to_owned(),
        evidence_excerpt: "dentist on July 15 at 7".to_owned(),
        observed_at: 1_783_000_000,
    }
}

pub fn store_candidate_observed(
    store: &Store,
    spec: (CandidateKind, &str, &str),
    observed_at: i64,
) -> CandidateId {
    let mut draft = lifecycle_draft(spec.0, spec.1, spec.2);
    draft.observed_at = observed_at;
    store.create_candidate(draft).expect("create candidate")
}

pub fn make_creating(store: &Store, candidate_id: &CandidateId, observed_at: i64) {
    store
        .transition_candidate(
            candidate_id,
            CandidateState::CreatingExternal,
            "start",
            observed_at,
        )
        .expect("transition creating");
}

pub fn make_visible(store: &Store, candidate_id: &CandidateId, observed_at: i64) {
    make_creating(store, candidate_id, observed_at);
    store
        .transition_candidate(
            candidate_id,
            CandidateState::Visible,
            "visible",
            observed_at + 1,
        )
        .expect("transition visible");
}
