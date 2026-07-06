#[path = "support/provider_usage.rs"]
mod provider_usage_support;

use morrow_storage::{ProviderUsageWindowKey, Store};
use provider_usage_support::{fresh_store, record_route, RouteFixture, NOW_UNIX_SECONDS};

#[test]
fn provider_usage_report_sorts_equal_count_aggregate_ties_by_id() {
    // Given: provider, model, and prompt groups with matching total outcome counts.
    let (_dir, _db_path, store) = fresh_store("provider-usage-aggregate-ties.sqlite");
    for (provider, model, prompt, route_fingerprint) in [
        (
            "zeta-provider",
            "model-b",
            "prompt-b",
            "usage-tie-zeta-model-b-prompt-b",
        ),
        (
            "zeta-provider",
            "model-b",
            "prompt-b",
            "usage-tie-zeta-model-b-prompt-b-extra",
        ),
        (
            "zeta-provider",
            "model-d",
            "prompt-d",
            "usage-tie-zeta-model-d-prompt-d",
        ),
        (
            "zeta-provider",
            "model-d",
            "prompt-d",
            "usage-tie-zeta-model-d-prompt-d-extra",
        ),
        (
            "alpha-provider",
            "model-c",
            "prompt-c",
            "usage-tie-alpha-model-c-prompt-c",
        ),
        (
            "alpha-provider",
            "model-c",
            "prompt-c",
            "usage-tie-alpha-model-c-prompt-c-extra",
        ),
        (
            "alpha-provider",
            "model-a",
            "prompt-z",
            "usage-tie-alpha-model-a-prompt-z",
        ),
        (
            "alpha-provider",
            "model-a",
            "prompt-a",
            "usage-tie-alpha-model-a-prompt-a",
        ),
    ] {
        record_route(
            &store,
            RouteFixture::quiet(route_fingerprint)
                .provider(provider)
                .model(model)
                .prompt(prompt),
        );
    }

    // When: storage builds an all-time usage report.
    let report = usage_report(&store, ProviderUsageWindowKey::All);
    let alpha_provider = report
        .providers
        .iter()
        .find(|provider| provider.provider_id == "alpha-provider")
        .expect("alpha provider");
    let alpha_model = alpha_provider
        .models
        .iter()
        .find(|model| model.model_id == "model-a")
        .expect("alpha model-a");

    // Then: equal-count ties use ascending provider, model, and prompt identifiers.
    assert_eq!(
        report
            .providers
            .iter()
            .map(|provider| provider.provider_id.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha-provider", "zeta-provider"]
    );
    assert_eq!(
        alpha_provider
            .models
            .iter()
            .map(|model| model.model_id.as_str())
            .collect::<Vec<_>>(),
        vec!["model-a", "model-c"]
    );
    assert_eq!(
        alpha_model
            .prompt_versions
            .iter()
            .map(|prompt| prompt.prompt_version.as_str())
            .collect::<Vec<_>>(),
        vec!["prompt-a", "prompt-z"]
    );
}

#[test]
fn provider_usage_report_sorts_equal_timestamp_recent_outcomes_by_newest_id() {
    // Given: same-timestamp outcomes inserted in ascending row-id order.
    let (_dir, _db_path, store) = fresh_store("provider-usage-recent-ties.sqlite");
    for (route_fingerprint, provider_id) in [
        ("usage-recent-same-time-first", "first-provider"),
        ("usage-recent-same-time-second", "second-provider"),
        ("usage-recent-same-time-third", "third-provider"),
    ] {
        record_route(
            &store,
            RouteFixture::quiet(route_fingerprint)
                .provider(provider_id)
                .observed_at(NOW_UNIX_SECONDS - 1),
        );
    }

    // When: storage builds an all-time usage report.
    let report = usage_report(&store, ProviderUsageWindowKey::All);

    // Then: equal created_at values are sorted by id DESC, observed via reverse insertion labels.
    assert_eq!(
        report
            .recent_outcomes
            .iter()
            .map(|outcome| outcome.provider_id.as_str())
            .collect::<Vec<_>>(),
        vec!["third-provider", "second-provider", "first-provider"]
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
