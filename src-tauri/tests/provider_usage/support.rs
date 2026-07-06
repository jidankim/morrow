use std::path::Path;

use morrow_storage::{
    CandidateKind, ProviderRouteCandidate, ProviderRouteOutcome, ProviderRouteOutcomeDraft,
    ProviderRouteRecordStatus, ProviderRouteSourceExcerptPolicy, Store,
    PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE,
};

pub const NOW_UNIX_SECONDS: i64 = 1_800_000_000;
pub const DAY_SECONDS: i64 = 24 * 60 * 60;
pub const SECRET_MARKER: &str = "sk-secret-provider-usage-token";
pub const PRIVATE_PATH_MARKER: &str = "/Users/private/morrow.sqlite";
pub const RAW_SQL_MARKER: &str = "SQLITE_NOT_A_DATABASE";

#[derive(Debug, Clone)]
pub struct RouteFixture<'a> {
    route_fingerprint: &'a str,
    provider_id: &'a str,
    model_id: &'a str,
    prompt_version: &'a str,
    route_label: &'a str,
    outcome: ProviderRouteOutcome,
    observed_at: i64,
}

impl<'a> RouteFixture<'a> {
    pub fn candidate(route_fingerprint: &'a str, confidence_millis: i64) -> Self {
        Self {
            route_fingerprint,
            provider_id: "provider-a",
            model_id: "model-a",
            prompt_version: "prompt-a",
            route_label: "calendar_route",
            outcome: ProviderRouteOutcome::Candidate(ProviderRouteCandidate {
                kind: CandidateKind::CalendarEvent,
                title: PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE.to_owned(),
                confidence_millis,
                normalized_time: "2026-07-02T18:00:00Z".to_owned(),
                evidence_excerpt: "safe short excerpt".to_owned(),
            }),
            observed_at: NOW_UNIX_SECONDS,
        }
    }

    pub fn quiet(route_fingerprint: &'a str) -> Self {
        Self {
            route_fingerprint,
            provider_id: "provider-a",
            model_id: "model-a",
            prompt_version: "prompt-a",
            route_label: "quiet_route",
            outcome: ProviderRouteOutcome::Quiet {
                quiet_reason: "provider_rejected:no_scheduling_signal".to_owned(),
            },
            observed_at: NOW_UNIX_SECONDS,
        }
    }

    pub const fn provider(mut self, provider_id: &'a str) -> Self {
        self.provider_id = provider_id;
        self
    }

    pub const fn model(mut self, model_id: &'a str) -> Self {
        self.model_id = model_id;
        self
    }

    pub const fn prompt(mut self, prompt_version: &'a str) -> Self {
        self.prompt_version = prompt_version;
        self
    }

    pub const fn route(mut self, route_label: &'a str) -> Self {
        self.route_label = route_label;
        self
    }

    pub fn quiet_reason(mut self, quiet_reason: &str) -> Self {
        self.outcome = ProviderRouteOutcome::Quiet {
            quiet_reason: quiet_reason.to_owned(),
        };
        self
    }

    pub const fn observed_at(mut self, observed_at: i64) -> Self {
        self.observed_at = observed_at;
        self
    }
}

pub fn fresh_store(name: &str) -> Result<(tempfile::TempDir, std::path::PathBuf, Store), String> {
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join(name);
    let store = Store::open(&db_path).map_err(|error| error.to_string())?;
    Ok((dir, db_path, store))
}

pub fn record_route(store: &Store, fixture: RouteFixture<'_>) -> Result<(), String> {
    let status = store
        .record_provider_route_outcome(route_draft(fixture))
        .map_err(|error| error.to_string())?;
    assert_eq!(status, ProviderRouteRecordStatus::Recorded);
    Ok(())
}

pub fn insert_malformed_row(db_path: &Path, observed_at: i64) -> Result<(), String> {
    let oversized_provider = "oversized-provider".repeat(10);
    let sql = format!(
        "INSERT INTO provider_route_outcomes
         (route_fingerprint, provider_route_contract_version,
          provider_candidate_schema_version, evidence_payload_hash, provider_id,
          model_id, prompt_version, source_excerpt_policy, reference_observed,
          reference_timezone, threshold_millis, parser_route, outcome_kind, candidate_kind,
          candidate_title, candidate_confidence_millis, candidate_normalized_time,
          candidate_evidence_excerpt, quiet_reason, created_at, updated_at)
         VALUES
         ('malformed-command-row', 'provider-route-ledger-v1', 'provider-candidate-schema-v2',
          'sha256:malformed-secret', '{oversized_provider}', 'model-private', 'prompt-private',
          'hide', '2026-07-01T09:30:00', 'Asia/Seoul', 700, 'private_route', 'quiet',
          NULL, NULL, NULL, NULL, NULL, 'private quiet reason', {observed_at}, {observed_at});"
    );
    std::process::Command::new("sqlite3")
        .arg("-batch")
        .arg(db_path)
        .arg(sql)
        .output()
        .map_err(|error| error.to_string())
        .and_then(|output| {
            if output.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        })
}

pub fn assert_payload_excludes_private_fields(json: &serde_json::Value) -> Result<(), String> {
    let text = serde_json::to_string(json).map_err(|error| error.to_string())?;
    assert_payload_excludes_private_text(&text);
    for field in [
        "quiet_reason",
        "candidate_evidence_excerpt",
        "evidence_payload_hash",
        "reference_observed",
        "reference_timezone",
        "candidate_title",
    ] {
        assert!(!text.contains(field), "payload leaked field {field}");
    }
    Ok(())
}

pub fn assert_payload_excludes_private_text(text: &str) {
    for private in [
        SECRET_MARKER,
        PRIVATE_PATH_MARKER,
        RAW_SQL_MARKER,
        "private quiet reason",
        "model-private",
        "prompt-private",
        "sha256:malformed-secret",
    ] {
        assert!(!text.contains(private), "payload leaked {private}");
    }
}

pub fn assert_safe_store_error(error: &str) {
    assert_eq!(error, "provider usage store unavailable");
    for private in [
        SECRET_MARKER,
        PRIVATE_PATH_MARKER,
        RAW_SQL_MARKER,
        "sqlite",
        "SQL",
        "/",
        ".sqlite",
    ] {
        assert!(!error.contains(private), "error leaked {private}");
    }
}

fn route_draft(fixture: RouteFixture<'_>) -> ProviderRouteOutcomeDraft {
    ProviderRouteOutcomeDraft {
        route_fingerprint: fixture.route_fingerprint.to_owned(),
        provider_route_contract_version: "provider-route-ledger-v1".to_owned(),
        provider_candidate_schema_version: "provider-candidate-schema-v2".to_owned(),
        evidence_payload_hash: format!("sha256:{}", fixture.route_fingerprint),
        provider_id: fixture.provider_id.to_owned(),
        model_id: fixture.model_id.to_owned(),
        prompt_version: fixture.prompt_version.to_owned(),
        profile_id: "list-reminders".to_owned(),
        profile_version: "list-reminders-v1".to_owned(),
        profile_schema_version: "single-reminder-title-v1".to_owned(),
        profile_policy_version: "explicit-only-disabled-v1".to_owned(),
        source_excerpt_policy: ProviderRouteSourceExcerptPolicy::Hide,
        reference_observed: "2026-07-01T09:30:00".to_owned(),
        reference_timezone: "Asia/Seoul".to_owned(),
        threshold_millis: 700,
        parser_route: fixture.route_label.to_owned(),
        outcome: fixture.outcome,
        observed_at: fixture.observed_at,
    }
}
