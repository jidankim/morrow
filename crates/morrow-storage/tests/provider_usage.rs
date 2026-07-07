#[path = "support/provider_usage.rs"]
mod provider_usage_support;

use morrow_storage::{
    CandidateKind, ProviderRouteCandidate, ProviderRouteOutcome,
    PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE,
};
use morrow_storage::{ProviderUsageWindowKey, Store};
use provider_usage_support::{fresh_store, record_route, RouteFixture, NOW_UNIX_SECONDS};

const DAY_SECONDS: i64 = 24 * 60 * 60;

#[test]
fn provider_usage_report_groups_windows_and_recent_outcomes() {
    // Given: provider-route outcomes across inclusive window boundaries.
    let (_dir, _db_path, store) = fresh_store("provider-usage-windows.sqlite");
    let start_7d = NOW_UNIX_SECONDS - (7 * DAY_SECONDS);
    let before_7d = start_7d - 1;
    let before_30d = NOW_UNIX_SECONDS - (30 * DAY_SECONDS) - 1;
    let before_90d = NOW_UNIX_SECONDS - (90 * DAY_SECONDS) - 1;
    record_route(
        &store,
        RouteFixture::candidate("usage-alpha-candidate", 800)
            .provider("alpha-provider")
            .observed_at(start_7d),
    );
    record_route(
        &store,
        RouteFixture::quiet("usage-alpha-quiet")
            .provider("alpha-provider")
            .observed_at(NOW_UNIX_SECONDS),
    );
    record_route(
        &store,
        RouteFixture::candidate("usage-beta-candidate", 500)
            .provider("beta-provider")
            .model("model-b")
            .prompt("prompt-b")
            .observed_at(NOW_UNIX_SECONDS - 10),
    );
    record_route(
        &store,
        RouteFixture::quiet("usage-outside-seven")
            .provider("outside-seven-provider")
            .model("model-c")
            .prompt("prompt-c")
            .observed_at(before_7d),
    );
    record_route(
        &store,
        RouteFixture::quiet("usage-outside-thirty")
            .provider("outside-thirty-provider")
            .model("model-d")
            .prompt("prompt-d")
            .observed_at(before_30d),
    );
    record_route(
        &store,
        RouteFixture::quiet("usage-all-only")
            .provider("all-only-provider")
            .model("model-e")
            .prompt("prompt-e")
            .observed_at(before_90d),
    );
    record_route(
        &store,
        RouteFixture::quiet("usage-after-now")
            .provider("future-provider")
            .model("model-f")
            .prompt("prompt-f")
            .observed_at(NOW_UNIX_SECONDS + 1),
    );

    // When: storage builds usage reports using an injected clock.
    let seven_day_report = usage_report(&store, ProviderUsageWindowKey::SevenDays);
    let thirty_day_report = usage_report(&store, ProviderUsageWindowKey::ThirtyDays);
    let ninety_day_report = usage_report(&store, ProviderUsageWindowKey::NinetyDays);
    let all_report = usage_report(&store, ProviderUsageWindowKey::All);

    // Then: counts, rates, sorting, averages, and inclusive bounds match the contract.
    assert_eq!(seven_day_report.generated_at_unix_seconds, NOW_UNIX_SECONDS);
    assert_eq!(seven_day_report.window.key.as_str(), "7d");
    assert_eq!(seven_day_report.window.start_unix_seconds, Some(start_7d));
    assert_eq!(seven_day_report.window.end_unix_seconds, NOW_UNIX_SECONDS);
    assert_eq!(seven_day_report.totals.total_outcomes, 3);
    assert_eq!(seven_day_report.totals.candidate_count, 2);
    assert_eq!(seven_day_report.totals.quiet_count, 1);
    assert_eq!(seven_day_report.totals.candidate_rate, 2.0 / 3.0);
    assert_eq!(seven_day_report.totals.quiet_rate, 1.0 / 3.0);
    assert_eq!(
        seven_day_report
            .providers
            .iter()
            .map(|provider| provider.provider_id.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha-provider", "beta-provider"]
    );
    assert_eq!(seven_day_report.providers[0].total_outcomes, 2);
    assert_eq!(seven_day_report.providers[0].candidate_count, 1);
    assert_eq!(seven_day_report.providers[0].quiet_count, 1);
    assert_eq!(seven_day_report.providers[0].average_confidence, Some(0.8));
    assert_eq!(seven_day_report.providers[0].models[0].model_id, "model-a");
    assert_eq!(
        seven_day_report.providers[0].models[0].prompt_versions[0].prompt_version,
        "prompt-a"
    );
    assert_eq!(
        seven_day_report.recent_outcomes[0].provider_id,
        "alpha-provider"
    );
    assert_eq!(seven_day_report.recent_outcomes[0].outcome_kind, "quiet");
    assert_eq!(
        seven_day_report.recent_outcomes[0].route_label,
        "parser_pattern"
    );
    assert_eq!(seven_day_report.recent_outcomes[1].confidence, Some(0.5));
    assert_eq!(thirty_day_report.totals.total_outcomes, 4);
    assert_eq!(ninety_day_report.totals.total_outcomes, 5);
    assert_eq!(all_report.window.key.as_str(), "all");
    assert_eq!(all_report.window.start_unix_seconds, None);
    assert_eq!(all_report.totals.total_outcomes, 6);
    assert!(!all_report
        .providers
        .iter()
        .any(|provider| provider.provider_id == "future-provider"));
}

#[test]
fn provider_usage_report_returns_empty_report_when_no_rows() {
    // Given: an empty provider-route ledger.
    let (_dir, _db_path, store) = fresh_store("provider-usage-empty.sqlite");

    // When: storage builds a bounded usage report.
    let report = usage_report(&store, ProviderUsageWindowKey::SevenDays);

    // Then: the report has zero counts and zero rates.
    assert_eq!(report.totals.total_outcomes, 0);
    assert_eq!(report.totals.candidate_count, 0);
    assert_eq!(report.totals.quiet_count, 0);
    assert_eq!(report.totals.candidate_rate, 0.0);
    assert_eq!(report.totals.quiet_rate, 0.0);
    assert!(report.providers.is_empty());
    assert!(report.recent_outcomes.is_empty());
}

#[test]
fn provider_usage_report_caps_recent_outcomes_at_25() {
    // Given: more than 25 provider-route outcomes in the selected window.
    let (_dir, _db_path, store) = fresh_store("provider-usage-recent-cap.sqlite");
    for index in 0..30 {
        record_route(
            &store,
            RouteFixture::quiet(&format!("usage-recent-{index}"))
                .provider("cap-provider")
                .model("model-cap")
                .prompt("prompt-cap")
                .observed_at(NOW_UNIX_SECONDS - i64::from(index)),
        );
    }

    // When: storage builds an all-time usage report.
    let report = usage_report(&store, ProviderUsageWindowKey::All);

    // Then: recent rows are capped at 25 and sorted by created_at DESC, id DESC.
    assert_eq!(report.totals.total_outcomes, 30);
    assert_eq!(report.recent_outcomes.len(), 25);
    assert_eq!(
        report
            .recent_outcomes
            .iter()
            .map(|outcome| outcome.created_at_unix_seconds)
            .take(3)
            .collect::<Vec<_>>(),
        vec![NOW_UNIX_SECONDS, NOW_UNIX_SECONDS - 1, NOW_UNIX_SECONDS - 2]
    );
}

fn usage_report(
    store: &Store,
    window_key: ProviderUsageWindowKey,
) -> morrow_storage::ProviderUsageReport {
    store
        .provider_usage_report(window_key, NOW_UNIX_SECONDS)
        .expect("usage report")
}

impl<'a> RouteFixture<'a> {
    fn candidate(route_fingerprint: &'a str, confidence_millis: i64) -> Self {
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
}
