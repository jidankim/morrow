use crate::app_shell::{menu_model, sync_now_event_allowed, AppShellState, AppState};
use tauri::{
    menu::{Menu, MenuBuilder, MenuItem, Submenu, SubmenuBuilder},
    AppHandle, Emitter, Manager, Runtime,
};

const MENU_PAUSE_RESUME_ID: &str = "morrow_pause_resume";
const MENU_SYNC_NOW_ID: &str = "morrow_sync_now";
const MENU_STATUS_ID: &str = "morrow_status";
const MENU_PENDING_ID: &str = "morrow_pending_count";
const MENU_SETTINGS_ID: &str = "morrow_settings";
const MENU_OPEN_CALENDAR_ID: &str = "morrow_open_calendar";
const MENU_OPEN_REMINDERS_ID: &str = "morrow_open_reminders";
const MENU_TOGGLE_AUTOMATIC_SYNC_ID: &str = "morrow_toggle_automatic_sync";
const MENU_QUIT_ID: &str = "morrow_quit";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeMenuSection {
    Morrow,
    Edit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StandardEditMenuItem {
    Undo,
    Redo,
    Separator,
    Cut,
    Copy,
    Paste,
    SelectAll,
}

const NATIVE_MENU_SECTIONS: [NativeMenuSection; 2] =
    [NativeMenuSection::Morrow, NativeMenuSection::Edit];
const STANDARD_EDIT_MENU_ITEMS: [StandardEditMenuItem; 7] = [
    StandardEditMenuItem::Undo,
    StandardEditMenuItem::Redo,
    StandardEditMenuItem::Separator,
    StandardEditMenuItem::Cut,
    StandardEditMenuItem::Copy,
    StandardEditMenuItem::Paste,
    StandardEditMenuItem::SelectAll,
];

pub fn install_menu(app: &AppHandle, state: &AppShellState) -> tauri::Result<()> {
    let app_menu = build_app_menu(app, state)?;
    app.set_menu(app_menu).map(|_| ())
}

fn build_app_menu<R: Runtime>(app: &AppHandle<R>, state: &AppShellState) -> tauri::Result<Menu<R>> {
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
    let toggle_automatic_sync = MenuItem::with_id(
        app,
        MENU_TOGGLE_AUTOMATIC_SYNC_ID,
        menu.automatic_sync_label,
        true,
        None::<&str>,
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
        .item(&toggle_automatic_sync)
        .separator()
        .item(&open_calendar)
        .item(&open_reminders)
        .separator()
        .item(&quit)
        .build()?;
    let mut app_menu = MenuBuilder::new(app);
    for section in NATIVE_MENU_SECTIONS {
        match section {
            NativeMenuSection::Morrow => {
                app_menu = app_menu.item(&morrow_menu);
            }
            NativeMenuSection::Edit => {
                let edit_menu = build_standard_edit_menu(app)?;
                app_menu = app_menu.item(&edit_menu);
            }
        }
    }
    app_menu.build()
}

fn build_standard_edit_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
    let mut edit_menu = SubmenuBuilder::new(app, "Edit");
    for item in STANDARD_EDIT_MENU_ITEMS {
        match item {
            StandardEditMenuItem::Undo => {
                edit_menu = edit_menu.undo();
            }
            StandardEditMenuItem::Redo => {
                edit_menu = edit_menu.redo();
            }
            StandardEditMenuItem::Separator => {
                edit_menu = edit_menu.separator();
            }
            StandardEditMenuItem::Cut => {
                edit_menu = edit_menu.cut();
            }
            StandardEditMenuItem::Copy => {
                edit_menu = edit_menu.copy();
            }
            StandardEditMenuItem::Paste => {
                edit_menu = edit_menu.paste();
            }
            StandardEditMenuItem::SelectAll => {
                edit_menu = edit_menu.select_all();
            }
        }
    }
    edit_menu.build()
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
        MENU_TOGGLE_AUTOMATIC_SYNC_ID => {
            let _ = app_handle.emit("morrow://toggle-automatic-sync", ());
        }
        MENU_QUIT_ID => {
            app_handle.exit(0);
        }
        _ => {}
    });
}

#[cfg(test)]
mod tests {
    use super::{
        NativeMenuSection, StandardEditMenuItem, NATIVE_MENU_SECTIONS, STANDARD_EDIT_MENU_ITEMS,
    };

    #[test]
    fn native_menu_keeps_standard_copy_command_available() {
        assert!(
            NATIVE_MENU_SECTIONS.contains(&NativeMenuSection::Edit),
            "native menu should include the standard Edit submenu so Cmd+C dispatches Copy"
        );
        assert!(
            STANDARD_EDIT_MENU_ITEMS.contains(&StandardEditMenuItem::Copy),
            "standard Edit menu items should include Copy"
        );
    }
}
