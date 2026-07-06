use std::sync::Mutex;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AppMode {
    Scanning,
    Paused,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AutomaticSyncStatusLabel {
    #[serde(rename = "Off")]
    Off,
    #[serde(rename = "On")]
    On,
    #[serde(rename = "Cooling Down")]
    CoolingDown,
    #[serde(rename = "Needs Action")]
    NeedsAction,
}

impl AutomaticSyncStatusLabel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::On => "On",
            Self::CoolingDown => "Cooling Down",
            Self::NeedsAction => "Needs Action",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppShellState {
    pub mode: AppMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub onboarding_complete: bool,
    pub pending_proposal_count: u32,
    pub sync_now_running: bool,
    pub automatic_sync_enabled: bool,
    pub automatic_sync_status_label: AutomaticSyncStatusLabel,
    pub automatic_sync_detail: String,
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
    pub automatic_sync_enabled: bool,
    pub automatic_sync_label: String,
    pub automatic_sync_detail: String,
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
            sync_now_running: false,
            automatic_sync_enabled: false,
            automatic_sync_status_label: AutomaticSyncStatusLabel::Off,
            automatic_sync_detail: "Automatic sync is off.".to_owned(),
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
            sync_now_running: false,
            automatic_sync_enabled: false,
            automatic_sync_status_label: AutomaticSyncStatusLabel::Off,
            automatic_sync_detail: "Automatic sync is off.".to_owned(),
        }
    }
}

pub fn menu_model(state: &AppShellState) -> MenuModel {
    match state.mode {
        AppMode::Scanning => scanning_menu_model(state),
        AppMode::Paused => MenuModel {
            status_kind: "paused".to_owned(),
            status_label: "Sync Now disabled".to_owned(),
            detail: "Sync Now is disabled on this Mac.".to_owned(),
            pause_resume_label: "Enable Sync Now".to_owned(),
            sync_now_enabled: false,
            pending_proposal_label: format_pending_proposal_count(state.pending_proposal_count),
            settings_label: "Settings".to_owned(),
            open_calendar_label: "Open Calendar".to_owned(),
            open_reminders_label: "Open Reminders".to_owned(),
            automatic_sync_enabled: state.automatic_sync_enabled,
            automatic_sync_label: automatic_sync_menu_label(state.automatic_sync_status_label),
            automatic_sync_detail: state.automatic_sync_detail.clone(),
            quit_label: "Quit Morrow".to_owned(),
        },
        AppMode::Error => MenuModel {
            status_kind: "error".to_owned(),
            status_label: "Error".to_owned(),
            detail: state
                .error_message
                .clone()
                .unwrap_or_else(|| "Morrow needs attention.".to_owned()),
            pause_resume_label: "Disable Sync Now".to_owned(),
            sync_now_enabled: state.onboarding_complete && !state.sync_now_running,
            pending_proposal_label: format_pending_proposal_count(state.pending_proposal_count),
            settings_label: "Settings".to_owned(),
            open_calendar_label: "Open Calendar".to_owned(),
            open_reminders_label: "Open Reminders".to_owned(),
            automatic_sync_enabled: state.automatic_sync_enabled,
            automatic_sync_label: automatic_sync_menu_label(state.automatic_sync_status_label),
            automatic_sync_detail: state.automatic_sync_detail.clone(),
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
            status_label: "Ready".to_owned(),
            detail: "Ready. Use Sync Now to reconcile calendars and scan selected chats."
                .to_owned(),
            pause_resume_label: "Disable Sync Now".to_owned(),
            sync_now_enabled: !state.sync_now_running,
            pending_proposal_label: format_pending_proposal_count(state.pending_proposal_count),
            settings_label: "Settings".to_owned(),
            open_calendar_label: "Open Calendar".to_owned(),
            open_reminders_label: "Open Reminders".to_owned(),
            automatic_sync_enabled: state.automatic_sync_enabled,
            automatic_sync_label: automatic_sync_menu_label(state.automatic_sync_status_label),
            automatic_sync_detail: state.automatic_sync_detail.clone(),
            quit_label: "Quit Morrow".to_owned(),
        };
    }

    MenuModel {
        status_kind: "setup-needed".to_owned(),
        status_label: "Setup needed".to_owned(),
        detail: "Complete setup and choose chats before Sync Now can scan.".to_owned(),
        pause_resume_label: "Disable Sync Now".to_owned(),
        sync_now_enabled: false,
        pending_proposal_label: format_pending_proposal_count(state.pending_proposal_count),
        settings_label: "Settings".to_owned(),
        open_calendar_label: "Open Calendar".to_owned(),
        open_reminders_label: "Open Reminders".to_owned(),
        automatic_sync_enabled: state.automatic_sync_enabled,
        automatic_sync_label: automatic_sync_menu_label(state.automatic_sync_status_label),
        automatic_sync_detail: state.automatic_sync_detail.clone(),
        quit_label: "Quit Morrow".to_owned(),
    }
}

fn automatic_sync_menu_label(status_label: AutomaticSyncStatusLabel) -> String {
    format!("Automatic Sync: {}", status_label.as_str())
}

fn format_pending_proposal_count(count: u32) -> String {
    if count >= 9 {
        "9+".to_owned()
    } else {
        count.to_string()
    }
}

#[cfg(test)]
mod tests;
