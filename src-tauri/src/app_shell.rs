use std::sync::Mutex;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AppMode {
    Scanning,
    Paused,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppShellState {
    pub mode: AppMode,
    pub error_message: Option<String>,
    pub onboarding_complete: bool,
    pub pending_proposal_count: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MenuModel {
    pub status_kind: String,
    pub status_label: String,
    pub detail: String,
    pub pause_resume_label: String,
    pub sync_now_enabled: bool,
    pub pending_proposal_label: String,
    pub settings_label: String,
    pub open_calendar_label: String,
    pub open_reminders_label: String,
    pub quit_label: String,
}

#[derive(Debug)]
pub struct AppState {
    state: Mutex<AppShellState>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(AppShellState::default()),
        }
    }

    pub fn snapshot(&self) -> Result<AppShellState, String> {
        let guard = self
            .state
            .lock()
            .map_err(|_| "app shell state lock is poisoned".to_owned())?;
        Ok(guard.clone())
    }

    pub fn replace(&self, next_state: AppShellState) -> Result<AppShellState, String> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| "app shell state lock is poisoned".to_owned())?;
        *guard = next_state;
        Ok(guard.clone())
    }

    pub fn toggle_pause_resume(&self) -> Result<AppShellState, String> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| "app shell state lock is poisoned".to_owned())?;
        *guard = match guard.mode {
            AppMode::Scanning | AppMode::Error => guard.pause(),
            AppMode::Paused => guard.resume(),
        };
        Ok(guard.clone())
    }
}

impl Default for AppShellState {
    fn default() -> Self {
        Self {
            mode: AppMode::Scanning,
            error_message: None,
            onboarding_complete: false,
            pending_proposal_count: 0,
        }
    }
}

impl AppShellState {
    pub fn pause(&self) -> Self {
        Self {
            mode: AppMode::Paused,
            error_message: None,
            ..self.clone()
        }
    }

    pub fn resume(&self) -> Self {
        Self {
            mode: AppMode::Scanning,
            error_message: None,
            ..self.clone()
        }
    }

    pub fn lock_error(message: String) -> Self {
        Self {
            mode: AppMode::Error,
            error_message: Some(message),
            onboarding_complete: false,
            pending_proposal_count: 0,
        }
    }
}

pub fn menu_model(state: &AppShellState) -> MenuModel {
    match state.mode {
        AppMode::Scanning => scanning_menu_model(state),
        AppMode::Paused => MenuModel {
            status_kind: "paused".to_owned(),
            status_label: "Paused".to_owned(),
            detail: "Scanning is paused on this Mac.".to_owned(),
            pause_resume_label: "Resume".to_owned(),
            sync_now_enabled: false,
            pending_proposal_label: format_pending_proposal_count(state.pending_proposal_count),
            settings_label: "Settings".to_owned(),
            open_calendar_label: "Open Calendar".to_owned(),
            open_reminders_label: "Open Reminders".to_owned(),
            quit_label: "Quit Morrow".to_owned(),
        },
        AppMode::Error => MenuModel {
            status_kind: "error".to_owned(),
            status_label: "Error".to_owned(),
            detail: state
                .error_message
                .clone()
                .unwrap_or_else(|| "Morrow needs attention.".to_owned()),
            pause_resume_label: "Resume".to_owned(),
            sync_now_enabled: state.onboarding_complete,
            pending_proposal_label: format_pending_proposal_count(state.pending_proposal_count),
            settings_label: "Settings".to_owned(),
            open_calendar_label: "Open Calendar".to_owned(),
            open_reminders_label: "Open Reminders".to_owned(),
            quit_label: "Quit Morrow".to_owned(),
        },
    }
}

pub fn sync_now_event_allowed(state: &AppShellState) -> bool {
    menu_model(state).sync_now_enabled
}

fn scanning_menu_model(state: &AppShellState) -> MenuModel {
    if state.onboarding_complete {
        return MenuModel {
            status_kind: "scanning".to_owned(),
            status_label: "Scanning".to_owned(),
            detail: "Ready to reconcile calendars, then scan selected chats.".to_owned(),
            pause_resume_label: "Pause".to_owned(),
            sync_now_enabled: true,
            pending_proposal_label: format_pending_proposal_count(state.pending_proposal_count),
            settings_label: "Settings".to_owned(),
            open_calendar_label: "Open Calendar".to_owned(),
            open_reminders_label: "Open Reminders".to_owned(),
            quit_label: "Quit Morrow".to_owned(),
        };
    }

    MenuModel {
        status_kind: "setup-needed".to_owned(),
        status_label: "Setup needed".to_owned(),
        detail: "Complete setup and choose chats before Sync Now can scan.".to_owned(),
        pause_resume_label: "Pause".to_owned(),
        sync_now_enabled: false,
        pending_proposal_label: format_pending_proposal_count(state.pending_proposal_count),
        settings_label: "Settings".to_owned(),
        open_calendar_label: "Open Calendar".to_owned(),
        open_reminders_label: "Open Reminders".to_owned(),
        quit_label: "Quit Morrow".to_owned(),
    }
}

fn format_pending_proposal_count(count: u32) -> String {
    if count >= 9 {
        "9+".to_owned()
    } else {
        count.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{menu_model, sync_now_event_allowed, AppMode, AppShellState};

    #[test]
    fn menu_model_reports_setup_needed_by_default() {
        let state = AppShellState::default();

        let menu = menu_model(&state);

        assert_eq!(menu.status_kind, "setup-needed");
        assert_eq!(menu.status_label, "Setup needed");
        assert!(!menu.sync_now_enabled);
        assert!(!sync_now_event_allowed(&state));
        assert_eq!(menu.settings_label, "Settings");
        assert_eq!(menu.open_calendar_label, "Open Calendar");
        assert_eq!(menu.open_reminders_label, "Open Reminders");
        assert_eq!(menu.quit_label, "Quit Morrow");
        assert_eq!(menu.pending_proposal_label, "0");
    }

    #[test]
    fn menu_model_reports_scanning_after_setup_complete() {
        let state = AppShellState {
            onboarding_complete: true,
            pending_proposal_count: 12,
            ..AppShellState::default()
        };

        let menu = menu_model(&state);

        assert_eq!(menu.status_kind, "scanning");
        assert_eq!(menu.status_label, "Scanning");
        assert!(menu.sync_now_enabled);
        assert!(sync_now_event_allowed(&state));
        assert_eq!(menu.pending_proposal_label, "9+");
    }

    #[test]
    fn menu_model_disables_sync_when_paused() {
        let state = AppShellState {
            mode: AppMode::Paused,
            onboarding_complete: true,
            pending_proposal_count: 8,
            ..AppShellState::default()
        };

        let menu = menu_model(&state);

        assert_eq!(menu.status_label, "Paused");
        assert_eq!(menu.status_kind, "paused");
        assert!(!menu.sync_now_enabled);
        assert!(!sync_now_event_allowed(&state));
        assert_eq!(menu.pause_resume_label, "Resume");
        assert_eq!(menu.pending_proposal_label, "8");
    }

    #[test]
    fn menu_model_reports_error_detail_after_setup_complete() {
        let state = AppShellState {
            mode: AppMode::Error,
            error_message: Some("Full Disk Access is unavailable".to_owned()),
            onboarding_complete: true,
            ..AppShellState::default()
        };

        let menu = menu_model(&state);

        assert_eq!(menu.status_label, "Error");
        assert_eq!(menu.status_kind, "error");
        assert_eq!(menu.detail, "Full Disk Access is unavailable");
        assert!(menu.sync_now_enabled);
        assert!(sync_now_event_allowed(&state));
    }

    #[test]
    fn menu_model_gates_error_sync_until_setup_complete() {
        let state = AppShellState {
            mode: AppMode::Error,
            error_message: Some("Full Disk Access is unavailable".to_owned()),
            ..AppShellState::default()
        };

        let menu = menu_model(&state);

        assert_eq!(menu.status_label, "Error");
        assert!(!menu.sync_now_enabled);
        assert!(!sync_now_event_allowed(&state));
    }
}
