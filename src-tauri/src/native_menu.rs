use crate::app_shell::{menu_model, sync_now_event_allowed, AppShellState, AppState};
use tauri::{
    menu::{MenuBuilder, MenuItem, SubmenuBuilder},
    AppHandle, Emitter, Manager,
};

const MENU_PAUSE_RESUME_ID: &str = "morrow_pause_resume";
const MENU_SYNC_NOW_ID: &str = "morrow_sync_now";
const MENU_STATUS_ID: &str = "morrow_status";
const MENU_PENDING_ID: &str = "morrow_pending_count";
const MENU_SETTINGS_ID: &str = "morrow_settings";
const MENU_OPEN_CALENDAR_ID: &str = "morrow_open_calendar";
const MENU_OPEN_REMINDERS_ID: &str = "morrow_open_reminders";
const MENU_QUIT_ID: &str = "morrow_quit";

pub fn install_menu(app: &AppHandle, state: &AppShellState) -> tauri::Result<()> {
    let menu = menu_model(state);
    let status = MenuItem::with_id(app, MENU_STATUS_ID, menu.status_label, false, None::<&str>)?;
    let pending = MenuItem::with_id(
        app,
        MENU_PENDING_ID,
        format!("Pending proposals: {}", menu.pending_proposal_label),
        false,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(
        app,
        MENU_SETTINGS_ID,
        menu.settings_label,
        true,
        Some("CmdOrCtrl+,"),
    )?;
    let pause_resume = MenuItem::with_id(
        app,
        MENU_PAUSE_RESUME_ID,
        menu.pause_resume_label,
        true,
        Some("CmdOrCtrl+Shift+P"),
    )?;
    let sync_now = MenuItem::with_id(
        app,
        MENU_SYNC_NOW_ID,
        "Sync Now",
        menu.sync_now_enabled,
        Some("CmdOrCtrl+Shift+S"),
    )?;
    let open_calendar = MenuItem::with_id(
        app,
        MENU_OPEN_CALENDAR_ID,
        menu.open_calendar_label,
        true,
        None::<&str>,
    )?;
    let open_reminders = MenuItem::with_id(
        app,
        MENU_OPEN_REMINDERS_ID,
        menu.open_reminders_label,
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(
        app,
        MENU_QUIT_ID,
        menu.quit_label,
        true,
        Some("CmdOrCtrl+Q"),
    )?;
    let morrow_menu = SubmenuBuilder::new(app, "Morrow")
        .item(&status)
        .item(&pending)
        .separator()
        .item(&settings)
        .item(&pause_resume)
        .item(&sync_now)
        .separator()
        .item(&open_calendar)
        .item(&open_reminders)
        .separator()
        .item(&quit)
        .build()?;
    let app_menu = MenuBuilder::new(app).item(&morrow_menu).build()?;
    app.set_menu(app_menu).map(|_| ())
}

pub fn register_menu_events(app: &mut tauri::App) {
    app.on_menu_event(|app_handle, event| match event.id().0.as_str() {
        MENU_PAUSE_RESUME_ID => {
            if let Some(state) = app_handle.try_state::<AppState>() {
                let next_state = state
                    .toggle_pause_resume()
                    .unwrap_or_else(AppShellState::lock_error);
                let _ = install_menu(app_handle, &next_state);
                let _ = app_handle.emit("morrow://app-state", next_state);
            }
        }
        MENU_SYNC_NOW_ID => {
            if let Some(state) = app_handle.try_state::<AppState>() {
                if let Ok(current) = state.snapshot() {
                    if sync_now_event_allowed(&current) {
                        let _ = app_handle.emit("morrow://sync-now", menu_model(&current));
                    }
                }
            }
        }
        MENU_SETTINGS_ID => {
            let _ = app_handle.emit("morrow://open-settings", ());
        }
        MENU_OPEN_CALENDAR_ID => {
            let _ = app_handle.emit("morrow://open-calendar", ());
        }
        MENU_OPEN_REMINDERS_ID => {
            let _ = app_handle.emit("morrow://open-reminders", ());
        }
        MENU_QUIT_ID => {
            app_handle.exit(0);
        }
        _ => {}
    });
}
