use std::error::Error;
use std::path::{Component, Path, PathBuf};
#[cfg(test)]
use std::sync::Mutex;

const EVIDENCE_DIR: &str =
    ".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/local-runner";
const PHASE_EVIDENCE_DIR: &str = ".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval";
const LOCAL_RUNNER_ENV: &str = "MORROW_PHASE5_LOCAL_RUNNER_OUT_DIR";
#[cfg(test)]
static OUT_DIR_ENV_LOCK: Mutex<()> = Mutex::new(());

type PolicyResult<T> = Result<T, Box<dyn Error>>;

pub(super) fn evidence_root() -> PolicyResult<PathBuf> {
    match std::env::var_os(LOCAL_RUNNER_ENV) {
        Some(out_dir) => resolve_evidence_root(Some(Path::new(&out_dir))),
        None => resolve_evidence_root(None),
    }
}

fn resolve_evidence_root(raw_out_dir: Option<&Path>) -> PolicyResult<PathBuf> {
    let requested = raw_out_dir.unwrap_or_else(|| Path::new(EVIDENCE_DIR));
    if requested.components().any(disallowed_out_dir_component) {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "local-runner out-dir must not contain . or .. path components",
        )));
    }
    let root = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        workspace_root().join(requested)
    };
    validate_phase_local_runner_root(&root)?;
    Ok(root)
}

fn disallowed_out_dir_component(component: Component<'_>) -> bool {
    matches!(
        component,
        Component::ParentDir | Component::CurDir | Component::Prefix(_)
    )
}

fn validate_phase_local_runner_root(root: &Path) -> PolicyResult<()> {
    let phase_root = workspace_root().join(PHASE_EVIDENCE_DIR);
    let relative = root.strip_prefix(&phase_root).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "local-runner out-dir must be under Phase 5 trajectory evidence root",
        )
    })?;
    let mut segments = relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(segment) => segment.to_str(),
            Component::RootDir
            | Component::CurDir
            | Component::ParentDir
            | Component::Prefix(_) => None,
        });
    let first = segments.next().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "local-runner out-dir must be a dedicated child directory",
        )
    })?;
    if first.starts_with("local-runner") {
        return Ok(());
    }
    let second = segments.next();
    if first.ends_with("-smoke")
        && second.is_some_and(|segment| segment.starts_with("local-runner"))
    {
        return Ok(());
    }
    Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "local-runner out-dir must be a local-runner evidence directory",
    )))
}

fn workspace_root() -> PathBuf {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop();
    root.pop();
    root
}

#[cfg(test)]
pub(super) fn with_out_dir_env_lock<T>(run: impl FnOnce() -> T) -> T {
    let _guard = match OUT_DIR_ENV_LOCK.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    run()
}

#[cfg(test)]
mod tests {
    use super::{evidence_root, resolve_evidence_root, with_out_dir_env_lock};
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn runner_out_dir_policy_accepts_gate_finalgate_and_smoke_paths() {
        let accepted = [
            ".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/local-runner",
            ".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/local-runner-gate",
            ".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/local-runner-finalgate-concurrency-a",
            ".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke/local-runner-source",
        ];

        for raw in accepted {
            assert!(
                resolve_evidence_root(Some(Path::new(raw))).is_ok(),
                "expected accepted local-runner out-dir: {raw}"
            );
        }
    }

    #[test]
    fn runner_out_dir_policy_rejects_unrelated_phase_dirs() {
        let rejected = [
            ".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/scoring",
            ".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-review",
            ".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke/scoring",
        ];

        for raw in rejected {
            assert!(
                resolve_evidence_root(Some(Path::new(raw))).is_err(),
                "expected rejected unrelated phase out-dir: {raw}"
            );
        }
    }

    #[test]
    fn runner_rejects_invalid_env_out_dir_before_evidence_write() {
        with_out_dir_env_lock(|| {
            let previous = std::env::var_os(super::LOCAL_RUNNER_ENV);
            let outside = std::env::temp_dir().join(format!(
                "morrow-phase5-local-runner-invalid-env-{}",
                unique_run_id()
            ));
            std::env::set_var(super::LOCAL_RUNNER_ENV, &outside);

            let result = evidence_root();
            restore_env(previous);

            assert!(
                result.is_err(),
                "expected invalid env out-dir to be rejected"
            );
            assert!(
                !outside.exists(),
                "invalid env out-dir must not be created: {}",
                outside.display()
            );
        });
    }

    fn restore_env(previous: Option<std::ffi::OsString>) {
        match previous {
            Some(value) => std::env::set_var(super::LOCAL_RUNNER_ENV, value),
            None => std::env::remove_var(super::LOCAL_RUNNER_ENV),
        }
    }

    fn unique_run_id() -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        format!("{}-{nanos}", std::process::id())
    }

    #[test]
    fn runner_out_dir_policy_rejects_absolute_roots_outside_phase_evidence() {
        let rejected = [
            PathBuf::from("/tmp/morrow-phase5-local-runner-invalid"),
            PathBuf::from("/Users/morrow-phase5-local-runner-invalid"),
            PathBuf::from("/private/tmp/morrow-phase5-local-runner-invalid"),
        ];

        for raw in rejected {
            assert!(
                resolve_evidence_root(Some(raw.as_path())).is_err(),
                "expected rejected absolute out-dir: {}",
                raw.display()
            );
        }
    }
}
