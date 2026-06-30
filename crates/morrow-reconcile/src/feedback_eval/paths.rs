use std::env;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::{Component, Path, PathBuf};

/// Invalid synthetic artifact path from an eval/example environment variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticPathError {
    env_name: &'static str,
    reason: &'static str,
}

impl Display for SyntheticPathError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} {}", self.env_name, self.reason)
    }
}

impl Error for SyntheticPathError {}

/// Reads and validates an env path owned by local synthetic Morrow examples.
pub fn synthetic_path_from_env(env_name: &'static str) -> Result<PathBuf, SyntheticPathError> {
    let raw = env::var(env_name).map_err(|_| synthetic_path_error(env_name, "must be set"))?;
    let path = PathBuf::from(&raw);
    validate_synthetic_path(env_name, &path)?;
    Ok(path)
}

fn validate_synthetic_path(env_name: &'static str, path: &Path) -> Result<(), SyntheticPathError> {
    if !path.is_absolute() {
        return Err(synthetic_path_error(env_name, "must be absolute"));
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::CurDir | Component::Prefix(_)
        )
    }) {
        return Err(synthetic_path_error(env_name, "must not contain traversal"));
    }
    let raw = path.to_string_lossy();
    if !raw.starts_with("/tmp/morrow-") && !raw.starts_with("/private/tmp/morrow-") {
        return Err(synthetic_path_error(
            env_name,
            "must start with /tmp/morrow-",
        ));
    }
    if std::fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink()) {
        return Err(synthetic_path_error(env_name, "must not be a symlink"));
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| synthetic_path_error(env_name, "must have a parent"))?;
    let parent = parent
        .canonicalize()
        .map_err(|_| synthetic_path_error(env_name, "parent must exist"))?;
    let tmp_root = Path::new("/tmp")
        .canonicalize()
        .map_err(|_| synthetic_path_error(env_name, "/tmp must exist"))?;
    if parent.starts_with(&tmp_root) {
        Ok(())
    } else {
        Err(synthetic_path_error(
            env_name,
            "parent must stay under /tmp",
        ))
    }
}

const fn synthetic_path_error(env_name: &'static str, reason: &'static str) -> SyntheticPathError {
    SyntheticPathError { env_name, reason }
}
