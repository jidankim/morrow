use std::path::Path;
use std::process::Command;

use morrow_storage::{
    CandidateKind, ProviderRouteCandidate, ProviderRouteOutcome, ProviderRouteOutcomeDraft,
    ProviderRouteOutcomeKind, ProviderRouteRecordStatus, ProviderRouteSourceExcerptPolicy,
    ProviderRouteStoredOutcome, StorageError, Store, PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE,
};

const FIELD_SEPARATOR: &str = "\u{1f}";

fn fresh_store(name: &str) -> (tempfile::TempDir, std::path::PathBuf, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join(name);
    let store = Store::open(&db_path).expect("open store");
    (dir, db_path, store)
}

fn sqlite_rows(db_path: &Path, sql: &str) -> Vec<Vec<String>> {
    let output = Command::new("sqlite3")
        .arg("-batch")
        .arg("-noheader")
        .arg("-separator")
        .arg(FIELD_SEPARATOR)
        .arg(db_path)
        .arg(sql)
        .output()
        .expect("run sqlite3");
    assert!(
        output.status.success(),
        "sqlite3 failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.split(FIELD_SEPARATOR).map(str::to_owned).collect())
        .collect()
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

#[test]
fn provider_route_rejects_private_payload_and_unavailable_outcome() {
    // Given: a fresh provider-route ledger and malformed/non-cacheable attempts.
    let (_dir, db_path, store) = fresh_store("provider-route-privacy.sqlite");
    let mut provider_title = candidate_draft("sha256:provider-title-route");
    provider_title.outcome = ProviderRouteOutcome::Candidate(ProviderRouteCandidate {
        kind: CandidateKind::CalendarEvent,
        title: "Provider supplied title".to_owned(),
        confidence_millis: 910,
        normalized_time: "2026-07-02T18:00:00Z".to_owned(),
        evidence_excerpt: "dinner tomorrow at 6".to_owned(),
    });
    let mut full_message_excerpt = candidate_draft("sha256:full-message-route");
    full_message_excerpt.outcome = ProviderRouteOutcome::Candidate(ProviderRouteCandidate {
        kind: CandidateKind::CalendarEvent,
        title: PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE.to_owned(),
        confidence_millis: 910,
        normalized_time: "2026-07-02T18:00:00Z".to_owned(),
        evidence_excerpt: "From: Alice +15555550103\nTo: bob@example.com\nMaybe meet tomorrow?"
            .to_owned(),
    });
    let mut unavailable = quiet_draft("sha256:unavailable-route");
    unavailable.outcome = ProviderRouteOutcome::ProviderUnavailable;

    // When: storage receives a provider title, full-message-like data, and an unavailable outcome.
    let provider_title_err = store
        .record_provider_route_outcome(provider_title)
        .expect_err("provider-supplied title must be rejected");
    let full_message_err = store
        .record_provider_route_outcome(full_message_excerpt)
        .expect_err("full-message-like excerpt must be rejected");
    let unavailable_status = store
        .record_provider_route_outcome(unavailable)
        .expect("provider_unavailable is a non-recording no-op");
    let rows = sqlite_rows(
        &db_path,
        "SELECT route_fingerprint, outcome_kind FROM provider_route_outcomes ORDER BY id;",
    );

    // Then: private data is rejected and provider_unavailable leaves no ledger row.
    assert!(matches!(
        provider_title_err,
        StorageError::PrivacyViolation { .. }
    ));
    assert!(matches!(
        full_message_err,
        StorageError::PrivacyViolation { .. }
    ));
    assert_eq!(
        unavailable_status,
        ProviderRouteRecordStatus::SkippedProviderUnavailable
    );
    assert!(rows.is_empty());
    println!(
        "provider_route_failure_path provider_title_rejected=true full_message_excerpt_rejected=true unavailable_recorded=false"
    );
}

#[test]
fn provider_route_storage_dump_contains_only_normalized_outcomes() {
    // Given: a provider-route ledger with one candidate and one quiet outcome.
    let (_dir, db_path, store) = fresh_store("provider-route-dump.sqlite");
    store
        .record_provider_route_outcome(candidate_draft("sha256:dump-candidate"))
        .expect("record candidate");
    store
        .record_provider_route_outcome(quiet_draft("sha256:dump-quiet"))
        .expect("record quiet");

    // When: the durable provider-route rows are dumped for QA.
    let rows = sqlite_rows(
        &db_path,
        "SELECT route_fingerprint, outcome_kind, candidate_title, candidate_evidence_excerpt,
                quiet_reason
         FROM provider_route_outcomes
         ORDER BY id;",
    );

    // Then: the dump contains normalized local outcomes only.
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0],
        vec![
            "sha256:dump-candidate".to_owned(),
            "candidate".to_owned(),
            PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE.to_owned(),
            "dinner tomorrow at 6".to_owned(),
            String::new(),
        ]
    );
    assert_eq!(
        rows[1],
        vec![
            "sha256:dump-quiet".to_owned(),
            "quiet".to_owned(),
            String::new(),
            String::new(),
            "provider_rejected:no_scheduling_signal".to_owned(),
        ]
    );
    println!("provider_route_sanitized_rows={rows:?}");
}

#[test]
fn provider_route_privacy_summary_includes_provider_route_outcomes() {
    // Given: a provider-route candidate excerpt with no evidence or quiet-log rows.
    let (_dir, _db_path, store) = fresh_store("provider-route-privacy-summary.sqlite");
    store
        .record_provider_route_outcome(candidate_draft("sha256:privacy-summary"))
        .expect("record candidate");

    // When: privacy storage is summarized.
    let summary = store.privacy_summary().expect("privacy summary");

    // Then: the provider-route table participates in prohibited-column and excerpt checks.
    assert_eq!(summary.full_message_body_columns, 0);
    assert_eq!(summary.max_excerpt_len, "dinner tomorrow at 6".len());
}
