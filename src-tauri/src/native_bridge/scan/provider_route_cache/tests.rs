use morrow_storage::{
    CandidateKind, ProviderRouteCandidate, ProviderRouteOutcome, ProviderRouteOutcomeDraft,
    ProviderRouteSourceExcerptPolicy, ProviderRouteStoredOutcome, Store,
    PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE,
};

use super::record_provider_route_ledger_writes;

#[test]
fn provider_route_writes_deduplicate_same_scan_fingerprints() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join("provider-route-duplicate.sqlite");
    let store = Store::open(&db_path).map_err(|error| error.to_string())?;

    // When
    record_provider_route_ledger_writes(
        &store,
        vec![
            quiet_draft("sha256:same-scan-route"),
            candidate_draft("sha256:same-scan-route"),
        ],
    )
    .map_err(|error| error.to_string())?;
    let row = store
        .provider_route_outcome("sha256:same-scan-route")
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "missing provider route ledger row".to_owned())?;

    // Then
    match row.outcome {
        ProviderRouteStoredOutcome::Candidate(candidate) => {
            assert_eq!(candidate.kind, CandidateKind::CalendarEvent);
            assert_eq!(candidate.title, PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE);
        }
        ProviderRouteStoredOutcome::Quiet { quiet_reason } => {
            return Err(format!("expected candidate row, got quiet {quiet_reason}"));
        }
    }
    Ok(())
}

fn candidate_draft(route_fingerprint: &str) -> ProviderRouteOutcomeDraft {
    ProviderRouteOutcomeDraft {
        route_fingerprint: route_fingerprint.to_owned(),
        provider_route_contract_version: "provider-route-ledger-v1".to_owned(),
        provider_candidate_schema_version: "provider-candidate-schema-v3".to_owned(),
        evidence_payload_hash: "sha256:evidence".to_owned(),
        provider_id: "openai".to_owned(),
        model_id: "gpt-4.1-mini".to_owned(),
        prompt_version: "messages-event-v1".to_owned(),
        profile_id: "list-reminders".to_owned(),
        profile_version: "list-reminders-v1".to_owned(),
        profile_schema_version: "single-reminder-title-v1".to_owned(),
        profile_policy_version: "explicit-only-disabled-v1".to_owned(),
        source_excerpt_policy: ProviderRouteSourceExcerptPolicy::Hide,
        reference_observed: "2026-07-01T09:30:00[Asia/Seoul]".to_owned(),
        reference_timezone: "Asia/Seoul".to_owned(),
        threshold_millis: 700,
        parser_route: "parser_provider_route_ambiguous_calendar".to_owned(),
        outcome: ProviderRouteOutcome::Candidate(ProviderRouteCandidate {
            kind: CandidateKind::CalendarEvent,
            title: PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE.to_owned(),
            confidence_millis: 910,
            normalized_time: "2026-07-02T18:00:00[Asia/Seoul]".to_owned(),
            evidence_excerpt: "Source excerpt hidden by provider-route ledger.".to_owned(),
        }),
        observed_at: 1_783_000_000,
    }
}

fn quiet_draft(route_fingerprint: &str) -> ProviderRouteOutcomeDraft {
    let mut draft = candidate_draft(route_fingerprint);
    draft.outcome = ProviderRouteOutcome::Quiet {
        quiet_reason: "provider_anchor_not_current_message".to_owned(),
    };
    draft.observed_at = 1_783_000_001;
    draft
}
