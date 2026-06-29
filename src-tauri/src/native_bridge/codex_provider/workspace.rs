use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use super::error::CodexProviderError;

const TEMP_PREFIX: &str = "morrow-codex-provider";

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) struct CodexTempWorkspace {
    root: PathBuf,
    cwd: PathBuf,
    schema_path: PathBuf,
    output_path: PathBuf,
}

impl CodexTempWorkspace {
    pub(super) fn create() -> Result<Self, CodexProviderError> {
        let root = unique_temp_root();
        let cwd = root.join("cwd");
        fs::create_dir(&root).map_err(|_| CodexProviderError::WorkspaceUnavailable)?;
        fs::create_dir(&cwd).map_err(|_| CodexProviderError::WorkspaceUnavailable)?;
        Ok(Self {
            schema_path: root.join("schema.json"),
            output_path: root.join("last-message.json"),
            root,
            cwd,
        })
    }

    pub(super) fn cwd(&self) -> &Path {
        &self.cwd
    }

    pub(super) fn schema_path(&self) -> &Path {
        &self.schema_path
    }

    pub(super) fn output_path(&self) -> &Path {
        &self.output_path
    }
}

impl Drop for CodexTempWorkspace {
    fn drop(&mut self) {
        match fs::remove_dir_all(&self.root) {
            Ok(()) | Err(_) => {}
        }
    }
}

fn unique_temp_root() -> PathBuf {
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    std::env::temp_dir().join(format!(
        "{TEMP_PREFIX}-{}-{timestamp}-{counter}",
        std::process::id()
    ))
}
