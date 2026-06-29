use std::{
    fmt,
    path::{Path, PathBuf},
    time::Duration,
};

use super::workspace::CodexTempWorkspace;

const CODEX_EXECUTABLE: &str = "codex";
const CODEX_EXEC_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone)]
pub struct CodexExecRequest {
    pub(super) executable: PathBuf,
    pub(super) args: Vec<String>,
    pub(super) cwd: PathBuf,
    pub(super) schema_path: PathBuf,
    pub(super) output_path: PathBuf,
    pub(super) prompt: String,
    pub(super) timeout: Duration,
}

impl CodexExecRequest {
    pub(super) fn for_workspace(workspace: &CodexTempWorkspace, prompt: String) -> Self {
        let schema_path = workspace.schema_path().to_path_buf();
        let output_path = workspace.output_path().to_path_buf();
        let cwd = workspace.cwd().to_path_buf();
        let args = codex_exec_args(&cwd, &schema_path, &output_path);
        Self {
            executable: PathBuf::from(CODEX_EXECUTABLE),
            args,
            cwd,
            schema_path,
            output_path,
            prompt,
            timeout: CODEX_EXEC_TIMEOUT,
        }
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    pub fn schema_path(&self) -> &Path {
        &self.schema_path
    }

    pub fn output_path(&self) -> &Path {
        &self.output_path
    }

    pub const fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }
}

impl fmt::Debug for CodexExecRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CodexExecRequest")
            .field("executable", &self.executable)
            .field("args", &self.args)
            .field("cwd", &self.cwd)
            .field("schema_path", &self.schema_path)
            .field("output_path", &self.output_path)
            .field("prompt", &"<redacted>")
            .field("timeout", &self.timeout)
            .finish()
    }
}

pub(super) fn codex_exec_args(cwd: &Path, schema_path: &Path, output_path: &Path) -> Vec<String> {
    vec![
        "exec".to_owned(),
        "--json".to_owned(),
        "--ephemeral".to_owned(),
        "--ignore-user-config".to_owned(),
        "--ignore-rules".to_owned(),
        "--skip-git-repo-check".to_owned(),
        "--sandbox".to_owned(),
        "read-only".to_owned(),
        "-c".to_owned(),
        "approval_policy=\"never\"".to_owned(),
        "-C".to_owned(),
        cwd.display().to_string(),
        "--output-schema".to_owned(),
        schema_path.display().to_string(),
        "--output-last-message".to_owned(),
        output_path.display().to_string(),
        "-".to_owned(),
    ]
}
