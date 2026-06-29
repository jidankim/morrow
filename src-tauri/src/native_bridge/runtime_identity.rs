use serde::Serialize;
use std::env;

const DISPLAY_NAME: &str = "Morrow";
const BUNDLE_IDENTIFIER: &str = "dev.morrow.desktop";
const APP_BUNDLE_EXECUTABLE_MARKER: &str = ".app/Contents/MacOS/";
const APP_BUNDLE_EXTENSION: &str = ".app";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeIdentity {
    pub display_name: &'static str,
    pub bundle_identifier: &'static str,
    pub executable_path: String,
    pub settings_target_path: String,
    pub runtime_kind: RuntimeKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeKind {
    AppBundle,
    Binary,
}

pub fn runtime_identity_from_executable_path(executable_path: &str) -> RuntimeIdentity {
    let app_root_path = app_bundle_root_path(executable_path);
    let runtime_kind = match app_root_path {
        Some(_) => RuntimeKind::AppBundle,
        None => RuntimeKind::Binary,
    };
    let settings_target_path = match app_root_path {
        Some(path) => path,
        None => executable_path.to_owned(),
    };

    RuntimeIdentity {
        display_name: DISPLAY_NAME,
        bundle_identifier: BUNDLE_IDENTIFIER,
        executable_path: executable_path.to_owned(),
        settings_target_path,
        runtime_kind,
    }
}

#[tauri::command]
pub fn get_runtime_identity() -> Result<RuntimeIdentity, String> {
    let executable_path = env::current_exe()
        .map_err(|error| format!("Failed to resolve Morrow executable path: {error}"))?;
    let executable_path = executable_path.to_string_lossy();
    Ok(runtime_identity_from_executable_path(&executable_path))
}

fn app_bundle_root_path(executable_path: &str) -> Option<String> {
    let marker_index = executable_path.find(APP_BUNDLE_EXECUTABLE_MARKER)?;
    let app_extension_end = marker_index + APP_BUNDLE_EXTENSION.len();
    Some(executable_path[..app_extension_end].to_owned())
}
