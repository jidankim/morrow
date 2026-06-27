use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CrashLogRequest {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CrashLogReceipt {
    pub stored: bool,
}

#[tauri::command]
pub fn record_crash_log(_request: CrashLogRequest) -> Result<CrashLogReceipt, String> {
    Ok(CrashLogReceipt { stored: false })
}
