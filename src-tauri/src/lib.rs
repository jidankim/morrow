mod app_shell;
mod native_menu;

use app_shell::{menu_model, AppShellState, AppState, MenuModel};
use native_menu::{install_menu, register_menu_events};
use tauri::{AppHandle, State};

pub mod native_bridge;

#[tauri::command]
fn get_app_state(state: State<'_, AppState>) -> Result<AppShellState, String> {
    state.snapshot()
}

#[tauri::command]
fn set_app_shell_state(
    app: AppHandle,
    state: State<'_, AppState>,
    shell_state: AppShellState,
) -> Result<MenuModel, String> {
    let next_state = state.replace(shell_state)?;
    install_menu(&app, &next_state).map_err(|error| error.to_string())?;
    Ok(menu_model(&next_state))
}

pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .manage(AppState::new())
        .manage(native_bridge::NativeBridgeState::default())
        .invoke_handler(tauri::generate_handler![
            get_app_state,
            set_app_shell_state,
            native_bridge::get_native_permission_statuses,
            native_bridge::open_privacy_settings,
            native_bridge::store_morrow_token,
            native_bridge::read_morrow_token,
            native_bridge::delete_morrow_token,
            native_bridge::check_provider_auth,
            native_bridge::delete_morrow_data,
            native_bridge::check_provider_auth,
            native_bridge::discover_messages_chats,
            native_bridge::reconcile_now,
            native_bridge::scan_selected_chats,
            native_bridge::record_crash_log
        ])
        .setup(|app| {
            let state = AppShellState::default();
            install_menu(app.handle(), &state)?;
            register_menu_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
}
