use std::path::PathBuf;

use tauri::{AppHandle, Manager};

pub fn messages_database_path() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join("Library/Messages/chat.db"))
        .ok_or_else(|| "Messages discovery is unavailable on this system.".to_owned())
}

pub fn morrow_store_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.join("morrow.sqlite"))
        .map_err(|error| error.to_string())
}
