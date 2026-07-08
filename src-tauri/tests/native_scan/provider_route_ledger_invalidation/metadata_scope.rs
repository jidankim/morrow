use std::{path::Path, process::Command};

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::super::dependencies::{CandidateProvider, CountingProvider, RecordingProposalAdapter};
use super::super::message_sqlite::{
    provider_route_fingerprint_count, provider_route_outcome_count, provider_route_outcome_dump,
};
use super::{provider_route_request_with_options, scan_provider_route, ProviderRouteFixture};

pub(super) fn v1_exact_second_row_is_not_reused_under_v2() -> Result<(), String> {
    // Given
    let fixture = ProviderRouteFixture::new()?;
    seed_v1_exact_second_provider_route_row(&fixture.store_path)?;
    let request = provider_route_request_with_options(true, "Asia/Seoul", 1_782_352_400)?;
    let provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    scan_provider_route(&fixture, request, &provider, &adapter, &recorder)?;

    // Then
    assert_eq!(provider.calls(), 1);
    assert_eq!(provider_route_fingerprint_count(&fixture.store_path)?, 2);
    let dump = provider_route_outcome_dump(&fixture.store_path)?;
    assert!(
        dump.contains("provider-route-ledger-v1")
            && dump.contains("2026-06-25T10:53:20[Asia/Seoul]"),
        "seeded row must be a true v1 exact-second row: {dump}"
    );
    assert!(
        dump.contains("provider-route-ledger-v2")
            && dump.contains("2026-06-25[Asia/Seoul]|Asia/Seoul"),
        "v2 scan must write a local-day scoped row after missing v1: {dump}"
    );
    println!(
        "v1_true_exact_second_provider_calls={} fingerprint_rows={} ledger={}",
        provider.calls(),
        provider_route_fingerprint_count(&fixture.store_path)?,
        dump.trim()
    );
    Ok(())
}

#[test]
fn provider_route_ledger_model_provider_threshold_metadata_changes_invalidate() -> Result<(), String>
{
    assert_metadata_change_invalidates(
        "provider_id = 'legacy-provider'",
        "legacy-provider",
        "provider_id",
    )?;
    assert_metadata_change_invalidates("model_id = 'legacy-model'", "legacy-model", "model_id")?;
    assert_metadata_change_invalidates("threshold_millis = 700", "|700|", "threshold_millis")?;
    Ok(())
}

fn assert_metadata_change_invalidates(
    assignments: &str,
    stale_fragment: &str,
    scenario: &str,
) -> Result<(), String> {
    // Given
    let fixture = ProviderRouteFixture::new()?;
    let request = provider_route_request_with_options(true, "Asia/Seoul", 1_782_352_400)?;
    let provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;
    scan_provider_route(&fixture, request.clone(), &provider, &adapter, &recorder)?;

    // When
    rewrite_provider_route_metadata(&fixture.store_path, assignments)?;
    scan_provider_route(&fixture, request, &provider, &adapter, &recorder)?;

    // Then
    assert_eq!(provider.calls(), 2);
    assert_eq!(provider_route_outcome_count(&fixture.store_path)?, 1);
    let dump = provider_route_outcome_dump(&fixture.store_path)?;
    assert!(
        dump.contains("|native-bridge|deterministic|scan-v3|"),
        "{dump}"
    );
    assert!(
        dump.contains("|550|parser_provider_route_ambiguous_calendar|"),
        "{dump}"
    );
    assert!(!dump.contains(stale_fragment), "{dump}");
    println!(
        "{scenario}_invalidation provider_calls={} rows={} ledger={}",
        provider.calls(),
        provider_route_outcome_count(&fixture.store_path)?,
        dump.trim()
    );
    Ok(())
}

fn seed_v1_exact_second_provider_route_row(db_path: &Path) -> Result<(), String> {
    drop(morrow_storage::Store::open(db_path).map_err(|error| error.to_string())?);
    let evidence_payload_hash =
        "sha256:ffac48a4f3a1a91ca3faa6b734557cbd64973bdc6874dfd95e48861a8f1bc570";
    let reference_observed = "2026-06-25T10:53:20[Asia/Seoul]";
    let route_fingerprint = v1_route_fingerprint(evidence_payload_hash, reference_observed)?;
    run_sqlite(
        db_path,
        &format!(
            "INSERT INTO provider_route_outcomes
             (route_fingerprint, provider_route_contract_version,
              provider_candidate_schema_version, evidence_payload_hash, provider_id,
              model_id, prompt_version, profile_id, profile_version,
              profile_schema_version, profile_policy_version, source_excerpt_policy,
              reference_observed, reference_timezone, threshold_millis, parser_route, outcome_kind,
              candidate_kind, candidate_title, candidate_confidence_millis,
              candidate_normalized_time, candidate_evidence_excerpt, quiet_reason,
              created_at, updated_at)
             VALUES
             ({}, 'provider-route-ledger-v1', 'provider-candidate-schema-v3',
              {}, 'native-bridge', 'deterministic', 'scan-v3', 'list-reminders',
              'list-reminders-v1', 'single-reminder-title-v1', 'explicit-only-disabled-v1',
              'include', {}, 'Asia/Seoul', 550, 'parser_provider_route_ambiguous_calendar',
              'candidate', 'calendar_event', 'Messages event candidate', 800,
              '2026-06-26T15:00:00[Asia/Seoul]',
              'Source excerpt hidden by provider-route ledger.', NULL, 1782352400, 1782352400);",
            sql_text(&route_fingerprint),
            sql_text(evidence_payload_hash),
            sql_text(reference_observed)
        ),
    )
}

fn v1_route_fingerprint(
    evidence_payload_hash: &str,
    reference_observed: &str,
) -> Result<String, String> {
    let canonical = CanonicalRouteMetadata {
        evidence_payload_hash,
        model_id: "deterministic",
        parser_route: "parser_provider_route_ambiguous_calendar",
        prompt_version: "scan-v3",
        profile_id: "list-reminders",
        profile_policy_version: "explicit-only-disabled-v1",
        profile_schema_version: "single-reminder-title-v1",
        profile_version: "list-reminders-v1",
        provider_candidate_schema_version: "provider-candidate-schema-v3",
        provider_id: "native-bridge",
        provider_route_contract_version: "provider-route-ledger-v1",
        reference_observed,
        reference_timezone: "Asia/Seoul",
        source_excerpt_policy: "include",
        threshold_millis: 550,
    };
    let canonical_json = serde_json::to_vec(&canonical).map_err(|error| error.to_string())?;
    Ok(format!("sha256:{:x}", Sha256::digest(canonical_json)))
}

fn rewrite_provider_route_metadata(db_path: &Path, assignments: &str) -> Result<(), String> {
    run_sqlite(
        db_path,
        &format!("UPDATE provider_route_outcomes SET {assignments};"),
    )
}

fn run_sqlite(db_path: &Path, sql: &str) -> Result<(), String> {
    let output = Command::new("sqlite3")
        .arg(db_path)
        .arg(sql)
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

fn sql_text(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CanonicalRouteMetadata<'a> {
    evidence_payload_hash: &'a str,
    model_id: &'a str,
    parser_route: &'a str,
    prompt_version: &'a str,
    profile_id: &'a str,
    profile_policy_version: &'a str,
    profile_schema_version: &'a str,
    profile_version: &'a str,
    provider_candidate_schema_version: &'a str,
    provider_id: &'a str,
    provider_route_contract_version: &'a str,
    reference_observed: &'a str,
    reference_timezone: &'a str,
    source_excerpt_policy: &'a str,
    threshold_millis: i64,
}
