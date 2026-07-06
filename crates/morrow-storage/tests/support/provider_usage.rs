use morrow_storage::{
    CandidateKind, ProviderRouteCandidate, ProviderRouteOutcome, ProviderRouteOutcomeDraft,
    ProviderRouteRecordStatus, ProviderRouteSourceExcerptPolicy, Store,
    PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE,
};

pub const NOW_UNIX_SECONDS: i64 = 1_800_000_000;

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
            outcome: candidate_outcome(confidence_millis),
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
            outcome: quiet_outcome(),
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

    pub const fn observed_at(mut self, observed_at: i64) -> Self {
        self.observed_at = observed_at;
        self
    }
}

pub fn fresh_store(name: &str) -> (tempfile::TempDir, std::path::PathBuf, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join(name);
    let store = Store::open(&db_path).expect("open store");
    (dir, db_path, store)
}

pub fn route_draft(fixture: RouteFixture<'_>) -> ProviderRouteOutcomeDraft {
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

pub fn record_route(store: &Store, fixture: RouteFixture<'_>) {
    assert_eq!(
        store
            .record_provider_route_outcome(route_draft(fixture))
            .expect("record provider route"),
        ProviderRouteRecordStatus::Recorded
    );
}

fn candidate_outcome(confidence_millis: i64) -> ProviderRouteOutcome {
    ProviderRouteOutcome::Candidate(ProviderRouteCandidate {
        kind: CandidateKind::CalendarEvent,
        title: PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE.to_owned(),
        confidence_millis,
        normalized_time: "2026-07-02T18:00:00Z".to_owned(),
        evidence_excerpt: "safe short excerpt".to_owned(),
    })
}

fn quiet_outcome() -> ProviderRouteOutcome {
    ProviderRouteOutcome::Quiet {
        quiet_reason: "provider_rejected:no_scheduling_signal".to_owned(),
    }
}
