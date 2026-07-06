use std::path::Path;
use std::process::Command;

use morrow_storage::{
    CandidateKind, ProviderRouteCandidate, ProviderRouteOutcome, ProviderRouteRecordStatus,
    StorageError, PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE,
};

use super::{candidate_draft, fresh_store, quiet_draft};

const FIELD_SEPARATOR: &str = "\u{1f}";

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
    let mut unsafe_profile_metadata = quiet_draft("sha256:unsafe-profile-metadata");
    unsafe_profile_metadata.profile_policy_version = "call mom after reading raw note".to_owned();

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
    let profile_metadata_err = store
        .record_provider_route_outcome(unsafe_profile_metadata)
        .expect_err("profile metadata must be a bounded token");
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
    assert!(matches!(
        profile_metadata_err,
        StorageError::InvalidInput {
            field: "profile_policy_version",
            ..
        }
    ));
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
        "SELECT route_fingerprint, profile_id, profile_version, profile_schema_version,
                profile_policy_version, outcome_kind, candidate_title, candidate_evidence_excerpt,
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
            "list-reminders".to_owned(),
            "list-reminders-v1".to_owned(),
            "single-reminder-title-v1".to_owned(),
            "explicit-only-disabled-v1".to_owned(),
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
            "list-reminders".to_owned(),
            "list-reminders-v1".to_owned(),
            "single-reminder-title-v1".to_owned(),
            "explicit-only-disabled-v1".to_owned(),
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
