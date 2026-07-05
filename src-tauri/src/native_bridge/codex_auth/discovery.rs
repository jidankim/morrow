use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

const COMMON_HOME_CLI_SEARCH_PATHS: &[&str] = &[
    ".local/bin",
    ".local/share/bin",
    ".npm-packages/bin",
    ".npm-global/bin",
    ".pnpm-packages/bin",
    ".bun/bin",
    ".volta/bin",
    ".asdf/shims",
    ".local/share/mise/shims",
    "Library/pnpm",
    "bin",
];

const COMMON_SYSTEM_CLI_SEARCH_PATHS: &[&str] = &[
    "/opt/homebrew/bin",
    "/usr/local/bin",
    "/usr/bin",
    "/bin",
    "/usr/sbin",
    "/sbin",
];

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
    default_search_paths(std::env::var_os("PATH"), std::env::var_os("HOME"))
}

fn default_search_paths(path: Option<OsString>, home: Option<OsString>) -> Vec<PathBuf> {
    let mut search_paths: Vec<PathBuf> = path
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default();
    append_home_cli_search_paths(&mut search_paths, home.map(PathBuf::from));
    append_system_cli_search_paths(&mut search_paths);
    search_paths
}

fn append_home_cli_search_paths(search_paths: &mut Vec<PathBuf>, home: Option<PathBuf>) {
    let Some(home) = home.filter(|path| !path.as_os_str().is_empty()) else {
        return;
    };
    for suffix in COMMON_HOME_CLI_SEARCH_PATHS {
        append_unique_path(search_paths, home.join(suffix));
    }
    append_nvm_node_bins(search_paths, &home);
}

fn append_nvm_node_bins(search_paths: &mut Vec<PathBuf>, home: &Path) {
    let Ok(entries) = std::fs::read_dir(home.join(".nvm").join("versions").join("node")) else {
        return;
    };
    let mut node_bins: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_dir()))
        .map(|entry| entry.path().join("bin"))
        .collect();
    node_bins.sort();
    for path in node_bins {
        append_unique_path(search_paths, path);
    }
}

fn append_system_cli_search_paths(search_paths: &mut Vec<PathBuf>) {
    for path in COMMON_SYSTEM_CLI_SEARCH_PATHS {
        append_unique_path(search_paths, PathBuf::from(path));
    }
}

fn append_unique_path(search_paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !path.as_os_str().is_empty() && !search_paths.contains(&path) {
        search_paths.push(path);
    }
}
