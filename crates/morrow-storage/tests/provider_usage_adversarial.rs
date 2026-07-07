#[path = "support/provider_usage.rs"]
mod provider_usage_support;

use std::path::Path;
use std::process::Command;

use morrow_storage::{
    CandidateKind, ProviderRouteCandidate, ProviderRouteOutcome, ProviderUsageWindowKey,
    PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE,
};
use provider_usage_support::{
    fresh_store, record_route, route_draft, RouteFixture, NOW_UNIX_SECONDS,
};

#[test]
fn provider_usage_report_omits_malformed_identifiers_and_keeps_prompt_version_inert() {
    // Given: rejected malformed API input, a malformed stored row, and hostile prompt-version data.
    let (_dir, db_path, store) = fresh_store("provider-usage-adversarial.sqlite");
    let rejected_provider = store.record_provider_route_outcome(route_draft(
        RouteFixture::quiet("usage-rejected-empty-provider").provider(""),
    ));
    sqlite_exec(
        &db_path,
        "INSERT INTO provider_route_outcomes
         (route_fingerprint, provider_route_contract_version,
          provider_candidate_schema_version, evidence_payload_hash, provider_id,
          model_id, prompt_version, source_excerpt_policy, reference_observed,
          reference_timezone, threshold_millis, parser_route, outcome_kind,
          quiet_reason, created_at, updated_at)
         VALUES
         ('usage-direct-empty-prompt', 'provider-route-ledger-v1',
          'provider-candidate-schema-v2', 'sha256:direct', 'direct-provider',
          'model-a', '', 'hide', 'private-reference-observed',
          '/Users/private-zone', 700, 'quiet_route', 'quiet',
          'private quiet reason', 1800000000, 1800000000);",
    );
    record_route(
        &store,
        RouteFixture::candidate("usage-hostile-prompt", 700)
            .provider("safe-provider")
            .model("model-safe")
            .prompt("<script>alert('prompt-version')</script>"),
    );

    // When: storage builds the usage report.
    let report = store
        .provider_usage_report(ProviderUsageWindowKey::All, NOW_UNIX_SECONDS)
        .expect("adversarial usage report");
    let report_debug = format!("{report:#?}");

    // Then: malformed ids are rejected or omitted, and hostile text is inert report data.
    assert!(rejected_provider.is_err());
    assert_eq!(report.totals.total_outcomes, 1);
    assert_eq!(report.providers[0].provider_id, "safe-provider");
    assert_eq!(
        report.providers[0].models[0].prompt_versions[0].prompt_version,
        "<script>alert('prompt-version')</script>"
    );
    assert!(report_debug.contains("prompt-version"));
    assert!(!report_debug.contains("direct-provider"));
    assert_no_private_report_strings(&report_debug);
}

#[test]
fn provider_usage_report_snapshot() {
    // Given: a small provider-route ledger with candidate and quiet outcomes.
    let (_dir, _db_path, store) = fresh_store("provider-usage-snapshot.sqlite");
    record_route(
        &store,
        RouteFixture::candidate("usage-snapshot-candidate", 900)
            .provider("snapshot-provider")
            .model("snapshot-model")
            .prompt("snapshot-prompt")
            .observed_at(NOW_UNIX_SECONDS - 1),
    );
    record_route(
        &store,
        RouteFixture::quiet("usage-snapshot-quiet")
            .provider("snapshot-provider")
            .model("snapshot-model")
            .prompt("snapshot-prompt")
            .observed_at(NOW_UNIX_SECONDS),
    );

    // When: storage builds the manual QA snapshot report.
    let report = store
        .provider_usage_report(ProviderUsageWindowKey::SevenDays, NOW_UNIX_SECONDS)
        .expect("snapshot usage report");
    let report_debug = format!("{report:#?}");

    // Then: the snapshot is data-shaped and excludes private provider-route fields.
    for expected in [
        "ProviderUsageReport",
        "generated_at_unix_seconds",
        "ProviderUsageWindow",
        "SevenDays",
        "total_outcomes",
        "provider_id",
        "model_id",
        "prompt_version",
        "outcome_kind",
        "route_label",
        "candidate_count",
        "quiet_count",
        "average_confidence",
    ] {
        assert!(
            report_debug.contains(expected),
            "snapshot missing {expected}: {report_debug}"
        );
    }
    assert_no_private_report_strings(&report_debug);
    println!("provider_usage_report_snapshot={report_debug}");
}

#[test]
fn provider_usage_report_coarsens_parser_time_route_before_public_report() {
    // Given: a real parser-route label that embeds parser time and reference timezone.
    let (_dir, _db_path, store) = fresh_store("provider-usage-parser-time-route.sqlite");
    record_route(
        &store,
        RouteFixture::candidate("usage-parser-time-route", 875)
            .route("parser_time:2026-07-01T09:30:00[Asia/Seoul]"),
    );

    // When: storage builds the usage report.
    let report = store
        .provider_usage_report(ProviderUsageWindowKey::All, NOW_UNIX_SECONDS)
        .expect("parser-time usage report");
    let report_debug = format!("{report:#?}");

    // Then: the public report exposes only the approved coarse route label.
    assert_eq!(report.recent_outcomes[0].route_label, "parser_time");
    assert!(!report_debug.contains("parser_time:2026-07-01T09:30:00"));
    assert!(!report_debug.contains("2026-07-01T09:30:00"));
    assert_no_private_report_strings(&report_debug);
}

fn sqlite_exec(db_path: &Path, sql: &str) {
    let output = Command::new("sqlite3")
        .arg("-batch")
        .arg(db_path)
        .arg(sql)
        .output()
        .expect("run sqlite3");
    assert!(
        output.status.success(),
        "sqlite3 failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_no_private_report_strings(report: &str) {
    for banned in [
        "quiet_reason",
        "candidate_evidence_excerpt",
        "evidence_payload_hash",
        "reference_observed",
        "reference_timezone",
        "candidate_title",
        "Provider supplied title",
        "provider_rejected:no_scheduling_signal",
        "safe short excerpt",
        "Asia/Seoul",
        "/Users/",
        "token",
        "secret",
        "cookie",
    ] {
        assert!(
            !report.contains(banned),
            "report leaked banned string {banned}: {report}"
        );
    }
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

    const fn route(mut self, route_label: &'a str) -> Self {
        self.route_label = route_label;
        self
    }
}
