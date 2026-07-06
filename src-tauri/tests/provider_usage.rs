#[path = "provider_usage/support.rs"]
mod support;

use morrow_lib::native_bridge::{load_provider_usage_at, LoadProviderUsageRequest};
use support::{
    assert_payload_excludes_private_fields, assert_payload_excludes_private_text,
    assert_safe_store_error, fresh_store, insert_malformed_row, record_route, RouteFixture,
    DAY_SECONDS, NOW_UNIX_SECONDS, PRIVATE_PATH_MARKER, RAW_SQL_MARKER, SECRET_MARKER,
};

#[test]
fn provider_usage_command_serializes_camel_case_default_window_when_store_has_rows(
) -> Result<(), String> {
    // Given: a local store with provider usage rows and private storage-only fields.
    let (_dir, db_path, store) = fresh_store("provider-usage-command.sqlite")?;
    record_route(
        &store,
        RouteFixture::candidate("command-candidate", 875)
            .provider("alpha-provider")
            .model("gpt-safe")
            .prompt("prompt-v1")
            .route("calendar_route")
            .observed_at(NOW_UNIX_SECONDS),
    )?;
    record_route(
        &store,
        RouteFixture::quiet("command-quiet")
            .provider("alpha-provider")
            .model("gpt-safe")
            .prompt("prompt-v1")
            .route("quiet_route")
            .quiet_reason(SECRET_MARKER)
            .observed_at(NOW_UNIX_SECONDS - DAY_SECONDS),
    )?;

    // When: the native command helper loads the default report.
    let report = load_provider_usage_at(
        &db_path,
        LoadProviderUsageRequest::default(),
        NOW_UNIX_SECONDS,
    )?;
    let json = serde_json::to_value(&report).map_err(|error| error.to_string())?;

    // Then: JSON is camelCase, defaulted to 30d, and excludes private storage fields.
    assert_eq!(report.window.key, "30d");
    assert_eq!(
        report.window.start_unix_seconds,
        Some(NOW_UNIX_SECONDS - (30 * DAY_SECONDS))
    );
    assert_eq!(report.generated_at_unix_seconds, NOW_UNIX_SECONDS);
    assert_eq!(json["generatedAtUnixSeconds"], NOW_UNIX_SECONDS);
    assert_eq!(
        json["window"]["startUnixSeconds"],
        NOW_UNIX_SECONDS - (30 * DAY_SECONDS)
    );
    assert_eq!(json["totals"]["totalOutcomes"], 2);
    assert_eq!(json["providers"][0]["providerId"], "alpha-provider");
    assert_eq!(json["providers"][0]["models"][0]["modelId"], "gpt-safe");
    assert_eq!(
        json["providers"][0]["models"][0]["promptVersions"][0]["promptVersion"],
        "prompt-v1"
    );
    assert_eq!(json["recentOutcomes"][0]["routeLabel"], "parser_pattern");
    assert_payload_excludes_private_fields(&json)?;
    Ok(())
}

#[test]
fn provider_usage_command_returns_empty_report_when_store_has_no_rows() -> Result<(), String> {
    // Given: an initialized store with no provider-route rows.
    let (_dir, db_path, _store) = fresh_store("provider-usage-command-empty.sqlite")?;

    // When: the native command helper loads a supported explicit window.
    let report = load_provider_usage_at(
        &db_path,
        LoadProviderUsageRequest {
            window_key: Some("7d".to_owned()),
        },
        NOW_UNIX_SECONDS,
    )?;

    // Then: counts are zero and the explicit window is preserved.
    assert_eq!(report.window.key, "7d");
    assert_eq!(report.totals.total_outcomes, 0);
    assert_eq!(report.totals.candidate_count, 0);
    assert_eq!(report.totals.quiet_count, 0);
    assert!(report.providers.is_empty());
    assert!(report.recent_outcomes.is_empty());
    Ok(())
}

#[test]
fn provider_usage_command_rejects_invalid_window_without_private_detail() -> Result<(), String> {
    // Given: a valid store path and an unsupported window key.
    let (_dir, db_path, _store) = fresh_store("provider-usage-command-invalid-window.sqlite")?;

    // When: the native command helper parses the request.
    let error = load_provider_usage_at(
        &db_path,
        LoadProviderUsageRequest {
            window_key: Some("../90d".to_owned()),
        },
        NOW_UNIX_SECONDS,
    )
    .expect_err("unsupported window should fail");

    // Then: the error is sanitized and does not echo input.
    assert_eq!(error, "invalid provider usage window");
    assert!(!error.contains(".."));
    Ok(())
}

#[test]
fn provider_usage_command_sanitizes_corrupted_or_missing_store_errors() -> Result<(), String> {
    // Given: paths that force sqlite/open failures with private raw details.
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let corrupted_path = dir.path().join("corrupted-provider-usage.sqlite");
    std::fs::write(&corrupted_path, RAW_SQL_MARKER).map_err(|error| error.to_string())?;
    let file_parent = dir.path().join("file-parent");
    std::fs::write(&file_parent, PRIVATE_PATH_MARKER).map_err(|error| error.to_string())?;
    let missing_store_path = file_parent.join("morrow.sqlite");

    // When: the native command helper attempts to load unreadable stores.
    let corrupted_error = load_provider_usage_at(
        &corrupted_path,
        LoadProviderUsageRequest::default(),
        NOW_UNIX_SECONDS,
    )
    .expect_err("corrupted store should fail");
    let missing_error = load_provider_usage_at(
        &missing_store_path,
        LoadProviderUsageRequest::default(),
        NOW_UNIX_SECONDS,
    )
    .expect_err("missing store path with file parent should fail");

    // Then: both failures use the same safe message with no path, SQL, or secret material.
    assert_safe_store_error(&corrupted_error);
    assert_safe_store_error(&missing_error);
    Ok(())
}

#[test]
fn provider_usage_command_coarsens_hostile_route_labels() -> Result<(), String> {
    // Given: one valid hostile-label row plus one malformed direct SQL row.
    let (_dir, db_path, store) = fresh_store("provider-usage-command-malformed.sqlite")?;
    let hostile_label = "<script>alert('x')</script>";
    record_route(
        &store,
        RouteFixture::candidate("command-hostile", 700)
            .provider("provider-json")
            .model("model-json")
            .prompt("prompt-json")
            .route(hostile_label)
            .observed_at(NOW_UNIX_SECONDS),
    )?;
    insert_malformed_row(&db_path, NOW_UNIX_SECONDS)?;

    // When: the native command helper loads the report.
    let report = load_provider_usage_at(
        &db_path,
        LoadProviderUsageRequest {
            window_key: Some("all".to_owned()),
        },
        NOW_UNIX_SECONDS,
    )?;
    let json = serde_json::to_string(&report).map_err(|error| error.to_string())?;

    // Then: malformed rows are omitted and hostile route labels collapse to an approved value.
    assert_eq!(report.totals.total_outcomes, 1);
    assert_eq!(report.recent_outcomes[0].route_label, "unknown");
    assert!(json.contains("\"routeLabel\":\"unknown\""));
    assert!(!json.contains(hostile_label));
    assert!(!json.contains("oversized-provider"));
    assert_payload_excludes_private_text(&json);
    Ok(())
}

#[test]
fn provider_usage_command_coarsens_parser_time_route_before_json() -> Result<(), String> {
    // Given: a local store with a raw parser route that embeds time and timezone metadata.
    let (_dir, db_path, store) = fresh_store("provider-usage-command-parser-time.sqlite")?;
    record_route(
        &store,
        RouteFixture::candidate("command-parser-time", 875)
            .route("parser_time:2026-07-01T09:30:00[Asia/Seoul]")
            .observed_at(NOW_UNIX_SECONDS),
    )?;

    // When: the native command helper serializes the usage report.
    let report = load_provider_usage_at(
        &db_path,
        LoadProviderUsageRequest {
            window_key: Some("all".to_owned()),
        },
        NOW_UNIX_SECONDS,
    )?;
    let json = serde_json::to_string(&report).map_err(|error| error.to_string())?;

    // Then: JSON exposes only the approved coarse route label.
    assert_eq!(report.recent_outcomes[0].route_label, "parser_time");
    assert!(json.contains("\"routeLabel\":\"parser_time\""));
    assert!(!json.contains("parser_time:2026-07-01T09:30:00"));
    assert!(!json.contains("2026-07-01T09:30:00"));
    assert!(!json.contains("Asia/Seoul"));
    Ok(())
}

#[test]
fn provider_usage_command_rereads_store_and_uses_supplied_clock() -> Result<(), String> {
    // Given: an initially empty store and a bounded test clock.
    let (_dir, db_path, store) = fresh_store("provider-usage-command-reread.sqlite")?;
    let first_report = load_provider_usage_at(
        &db_path,
        LoadProviderUsageRequest::default(),
        NOW_UNIX_SECONDS,
    )?;
    record_route(
        &store,
        RouteFixture::quiet("command-reread")
            .provider("fresh-provider")
            .observed_at(NOW_UNIX_SECONDS + 5),
    )?;

    // When: the command helper is called again with a later clock.
    let second_report = load_provider_usage_at(
        &db_path,
        LoadProviderUsageRequest::default(),
        NOW_UNIX_SECONDS + 5,
    )?;

    // Then: no stale report is reused.
    assert_eq!(first_report.totals.total_outcomes, 0);
    assert_eq!(
        second_report.generated_at_unix_seconds,
        NOW_UNIX_SECONDS + 5
    );
    assert_eq!(second_report.totals.total_outcomes, 1);
    assert_eq!(second_report.providers[0].provider_id, "fresh-provider");
    Ok(())
}

#[test]
fn provider_usage_command_snapshot_prints_sanitized_json_and_errors() -> Result<(), String> {
    // Given: a report with camelCase fields plus a corrupted store error.
    let (_dir, db_path, store) = fresh_store("provider-usage-command-snapshot.sqlite")?;
    record_route(
        &store,
        RouteFixture::candidate("command-snapshot", 625)
            .provider("snapshot-provider")
            .model("snapshot-model")
            .prompt("snapshot-prompt")
            .route("snapshot_route")
            .observed_at(NOW_UNIX_SECONDS),
    )?;
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let corrupted_path = dir.path().join("corrupted-snapshot.sqlite");
    std::fs::write(&corrupted_path, RAW_SQL_MARKER).map_err(|error| error.to_string())?;

    // When: the command helper serializes success and sanitized failure shapes.
    let report = load_provider_usage_at(
        &db_path,
        LoadProviderUsageRequest::default(),
        NOW_UNIX_SECONDS,
    )?;
    let error = load_provider_usage_at(
        &corrupted_path,
        LoadProviderUsageRequest::default(),
        NOW_UNIX_SECONDS,
    )
    .expect_err("corrupted store should fail");
    let json = serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?;

    // Then: command-shaped QA can inspect the exact emitted contract.
    println!("provider_usage_command_snapshot_json={json}");
    println!("provider_usage_command_snapshot_error={error}");
    assert!(json.contains("\"generatedAtUnixSeconds\""));
    assert!(json.contains("\"totalOutcomes\""));
    assert!(json.contains("\"providerId\""));
    assert_safe_store_error(&error);
    Ok(())
}
