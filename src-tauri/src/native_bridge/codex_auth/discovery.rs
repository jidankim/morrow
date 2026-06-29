use std::path::{Path, PathBuf};

pub(super) fn resolve_executable(
    executable_name: &str,
    search_paths: &[PathBuf],
) -> Option<PathBuf> {
    let candidate = Path::new(executable_name);
    if candidate.components().count() > 1 {
        return executable_file(candidate).then(|| candidate.to_path_buf());
    }
    search_paths
        .iter()
        .map(|path| path.join(executable_name))
        .find(|path| executable_file(path))
}

fn executable_file(path: &Path) -> bool {
    path.metadata()
        .is_ok_and(|metadata| metadata.is_file() && executable_mode(&metadata))
}

#[cfg(unix)]
fn executable_mode(metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;

    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn executable_mode(_metadata: &std::fs::Metadata) -> bool {
    true
}

pub(super) fn search_paths_from_env() -> Vec<PathBuf> {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default()
}
