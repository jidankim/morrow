use std::{env, process::Command};

#[path = "native_scan_performance_smoke_support.rs"]
mod native_scan_performance_smoke_support;

use native_scan_performance_smoke_support::{
    run_native_scan_performance_smoke_child, BUDGET_LINE_PREFIX, SMOKE_CHILD_ENV,
    STORAGE_SQLITE_RUN_LOG_ENV,
};

#[test]
fn native_scan_performance_smoke() -> Result<(), String> {
    if env::var_os(SMOKE_CHILD_ENV).is_some() {
        return run_native_scan_performance_smoke_child();
    }

    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let run_log = dir.path().join("storage-sqlite-runs.log");

    // When
    let output = Command::new(env::current_exe().map_err(|error| error.to_string())?)
        .arg("--exact")
        .arg("native_scan_performance_smoke::native_scan_performance_smoke")
        .arg("--nocapture")
        .env(SMOKE_CHILD_ENV, "1")
        .env(STORAGE_SQLITE_RUN_LOG_ENV, &run_log)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "child smoke failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let budget_line = stdout
        .lines()
        .find(|line| line.starts_with(BUDGET_LINE_PREFIX))
        .ok_or_else(|| format!("child smoke did not print budget line: {stdout}"))?;
    println!("{budget_line}");

    // Then
    assert_budget_line_shape(budget_line)?;
    Ok(())
}

fn assert_budget_line_shape(line: &str) -> Result<(), String> {
    if !line.starts_with(BUDGET_LINE_PREFIX) {
        return Err(format!("missing final budget prefix: {line}"));
    }
    for field in [
        "provider_calls=",
        "sqlite_runs=",
        "provider_route_rows=",
        "external_created=",
        "second_scan_created=",
    ] {
        if !line.contains(field) {
            return Err(format!("budget line missing {field}: {line}"));
        }
    }
    Ok(())
}
