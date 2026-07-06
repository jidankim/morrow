use morrow_storage::{
    CandidateKind, ProviderRouteCandidate, ProviderRouteOutcome, ProviderRouteOutcomeDraft,
    ProviderRouteOutcomeKind, ProviderRouteRecordStatus, ProviderRouteSourceExcerptPolicy,
    ProviderRouteStoredOutcome, StorageError, Store, PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE,
};

#[path = "provider_routes/privacy.rs"]
mod privacy;

fn fresh_store(name: &str) -> (tempfile::TempDir, std::path::PathBuf, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join(name);
    let store = Store::open(&db_path).expect("open store");
    (dir, db_path, store)
}

fn candidate_draft(route_fingerprint: &str) -> ProviderRouteOutcomeDraft {
    ProviderRouteOutcomeDraft {
        route_fingerprint: route_fingerprint.to_owned(),
        provider_route_contract_version: "provider-route-ledger-v1".to_owned(),
        provider_candidate_schema_version: "provider-candidate-schema-v2".to_owned(),
        evidence_payload_hash: "sha256:abc123".to_owned(),
        provider_id: "openai".to_owned(),
        model_id: "gpt-4.1-mini".to_owned(),
        prompt_version: "messages-event-v1".to_owned(),
        profile_id: "list-reminders".to_owned(),
        profile_version: "list-reminders-v1".to_owned(),
        profile_schema_version: "single-reminder-title-v1".to_owned(),
        profile_policy_version: "explicit-only-disabled-v1".to_owned(),
        source_excerpt_policy: ProviderRouteSourceExcerptPolicy::Include,
        reference_observed: "2026-07-01T09:30:00".to_owned(),
        reference_timezone: "Asia/Seoul".to_owned(),
        threshold_millis: 700,
        parser_route: "parser_time:2026-07-01T09:30:00[Asia/Seoul]".to_owned(),
        outcome: ProviderRouteOutcome::Candidate(ProviderRouteCandidate {
            kind: CandidateKind::CalendarEvent,
            title: PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE.to_owned(),
            confidence_millis: 910,
            normalized_time: "2026-07-02T18:00:00Z".to_owned(),
            evidence_excerpt: "dinner tomorrow at 6".to_owned(),
        }),
        observed_at: 1_783_000_000,
    }
}

fn quiet_draft(route_fingerprint: &str) -> ProviderRouteOutcomeDraft {
    ProviderRouteOutcomeDraft {
        route_fingerprint: route_fingerprint.to_owned(),
        provider_route_contract_version: "provider-route-ledger-v1".to_owned(),
        provider_candidate_schema_version: "provider-candidate-schema-v2".to_owned(),
        evidence_payload_hash: "sha256:def456".to_owned(),
        provider_id: "openai".to_owned(),
        model_id: "gpt-4.1-mini".to_owned(),
        prompt_version: "messages-event-v1".to_owned(),
        profile_id: "list-reminders".to_owned(),
        profile_version: "list-reminders-v1".to_owned(),
        profile_schema_version: "single-reminder-title-v1".to_owned(),
        profile_policy_version: "explicit-only-disabled-v1".to_owned(),
        source_excerpt_policy: ProviderRouteSourceExcerptPolicy::Hide,
        reference_observed: "2026-07-01T09:30:00".to_owned(),
        reference_timezone: "Asia/Seoul".to_owned(),
        threshold_millis: 700,
        parser_route: "none".to_owned(),
        outcome: ProviderRouteOutcome::Quiet {
            quiet_reason: "provider_rejected:no_scheduling_signal".to_owned(),
        },
        observed_at: 1_783_000_010,
    }
}

#[test]
fn provider_route_records_and_finds_candidate_outcome() {
    // Given: a fresh provider-route ledger.
    let (_dir, _db_path, store) = fresh_store("provider-route-candidate.sqlite");

    // When: a privacy-normalized candidate outcome is recorded and looked up.
    let status = store
        .record_provider_route_outcome(candidate_draft("sha256:candidate-route"))
        .expect("record provider route");
    let found = store
        .provider_route_outcome("sha256:candidate-route")
        .expect("lookup provider route")
        .expect("candidate outcome");

    // Then: lookup returns the normalized candidate without provider-supplied title data.
    assert_eq!(status, ProviderRouteRecordStatus::Recorded);
    assert_eq!(found.route_fingerprint, "sha256:candidate-route");
    assert_eq!(found.profile_id, "list-reminders");
    assert_eq!(found.profile_version, "list-reminders-v1");
    assert_eq!(found.profile_schema_version, "single-reminder-title-v1");
    assert_eq!(found.profile_policy_version, "explicit-only-disabled-v1");
    assert_eq!(found.outcome_kind(), ProviderRouteOutcomeKind::Candidate);
    match found.outcome {
        ProviderRouteStoredOutcome::Candidate(candidate) => {
            assert_eq!(candidate.kind, CandidateKind::CalendarEvent);
            assert_eq!(candidate.title, PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE);
            assert_eq!(candidate.confidence_millis, 910);
            assert_eq!(candidate.normalized_time, "2026-07-02T18:00:00Z");
            assert_eq!(candidate.evidence_excerpt, "dinner tomorrow at 6");
        }
        ProviderRouteStoredOutcome::Quiet { quiet_reason } => {
            panic!("expected candidate outcome, got quiet reason {quiet_reason}");
        }
    }
}

#[test]
fn provider_route_records_and_finds_quiet_outcome() {
    // Given: a fresh provider-route ledger.
    let (_dir, _db_path, store) = fresh_store("provider-route-quiet.sqlite");

    // When: a stable quiet outcome is recorded and looked up.
    let status = store
        .record_provider_route_outcome(quiet_draft("sha256:quiet-route"))
        .expect("record provider route");
    let found = store
        .provider_route_outcome("sha256:quiet-route")
        .expect("lookup provider route")
        .expect("quiet outcome");

    // Then: lookup returns the stable quiet reason and no candidate fields.
    assert_eq!(status, ProviderRouteRecordStatus::Recorded);
    assert_eq!(found.outcome_kind(), ProviderRouteOutcomeKind::Quiet);
    match found.outcome {
        ProviderRouteStoredOutcome::Quiet { quiet_reason } => {
            assert_eq!(quiet_reason, "provider_rejected:no_scheduling_signal");
        }
        ProviderRouteStoredOutcome::Candidate(candidate) => {
            panic!("expected quiet outcome, got candidate {candidate:?}");
        }
    }
}

#[test]
fn provider_route_fingerprint_is_unique_and_changed_fingerprint_misses() {
    // Given: a recorded route outcome.
    let (_dir, _db_path, store) = fresh_store("provider-route-unique.sqlite");
    store
        .record_provider_route_outcome(candidate_draft("sha256:unique-route"))
        .expect("record provider route");

    // When: the same fingerprint is recorded again and a changed fingerprint is looked up.
    let duplicate = store.record_provider_route_outcome(candidate_draft("sha256:unique-route"));
    let changed = store
        .provider_route_outcome("sha256:changed-route")
        .expect("lookup changed route");

    // Then: route fingerprints are unique and a changed fingerprint invalidates the row.
    assert!(matches!(duplicate, Err(StorageError::Sqlite { .. })));
    assert!(changed.is_none());
}
