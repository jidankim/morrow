use std::{
    env,
    process::Command,
    sync::{Mutex, MutexGuard},
};

static STORAGE_SQLITE_RUN_LOG_ENV_LOCK: Mutex<()> = Mutex::new(());

pub(in super::super) fn lock_storage_sqlite_run_log_env() -> MutexGuard<'static, ()> {
    STORAGE_SQLITE_RUN_LOG_ENV_LOCK
        .lock()
        .expect("storage sqlite run log env lock")
}

pub(in super::super) fn run_current_test_in_child(
    test_name: &str,
    child_env: &'static str,
) -> Result<(), String> {
    let output = Command::new(env::current_exe().map_err(|error| error.to_string())?)
        .arg("--exact")
        .arg(test_name)
        .arg("--nocapture")
        .env(child_env, "1")
        .output()
        .map_err(|error| error.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    print!("{stdout}");
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "child test {test_name} failed\nstdout:\n{stdout}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}
