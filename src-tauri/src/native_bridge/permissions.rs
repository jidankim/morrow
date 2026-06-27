use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum PermissionKind {
    FullDiskAccess,
    Calendar,
    Reminders,
    Contacts,
    Notifications,
    FilesystemPath,
}

impl PermissionKind {
    pub const ALL: [Self; 6] = [
        Self::FullDiskAccess,
        Self::Calendar,
        Self::Reminders,
        Self::Contacts,
        Self::Notifications,
        Self::FilesystemPath,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::FullDiskAccess => "Full Disk Access",
            Self::Calendar => "Calendar",
            Self::Reminders => "Reminders",
            Self::Contacts => "Contacts",
            Self::Notifications => "Notifications",
            Self::FilesystemPath => "filesystem path access",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PermissionState {
    Granted,
    Denied,
    Unavailable,
}

impl PermissionState {
    pub const ALL: [Self; 3] = [Self::Granted, Self::Denied, Self::Unavailable];
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PermissionOutcome {
    Success,
    Warning,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PermissionStatus {
    pub kind: PermissionKind,
    pub state: PermissionState,
    pub outcome: PermissionOutcome,
    pub warning: Option<String>,
}

pub fn map_permission_status(kind: PermissionKind, state: PermissionState) -> PermissionStatus {
    match state {
        PermissionState::Granted => PermissionStatus {
            kind,
            state,
            outcome: PermissionOutcome::Success,
            warning: None,
        },
        PermissionState::Denied => PermissionStatus {
            kind,
            state,
            outcome: PermissionOutcome::Warning,
            warning: Some(format!("{} permission is denied.", kind.label())),
        },
        PermissionState::Unavailable => PermissionStatus {
            kind,
            state,
            outcome: PermissionOutcome::Unavailable,
            warning: Some(format!(
                "{} permission is unavailable on this surface.",
                kind.label()
            )),
        },
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PrivacySettingsPane {
    FullDiskAccess,
    Calendar,
    Reminders,
}

impl PrivacySettingsPane {
    const fn settings_url(self) -> &'static str {
        match self {
            Self::FullDiskAccess => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles"
            }
            Self::Calendar => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Calendars"
            }
            Self::Reminders => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Reminders"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpenPrivacySettingsRequest {
    pub pane: PrivacySettingsPane,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpenPrivacySettingsReceipt {
    pub pane: PrivacySettingsPane,
    pub opened: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenPrivacySettingsError {
    message: String,
}

impl OpenPrivacySettingsError {
    #[cfg(not(target_os = "macos"))]
    fn unavailable() -> Self {
        Self {
            message: "macOS privacy settings are unavailable on this platform".to_owned(),
        }
    }

    #[cfg(target_os = "macos")]
    fn command_failed(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for OpenPrivacySettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for OpenPrivacySettingsError {}

pub fn open_privacy_settings(
    request: OpenPrivacySettingsRequest,
) -> Result<OpenPrivacySettingsReceipt, OpenPrivacySettingsError> {
    open_settings_url(request.pane.settings_url())?;
    Ok(OpenPrivacySettingsReceipt {
        pane: request.pane,
        opened: true,
    })
}

#[cfg(target_os = "macos")]
fn open_settings_url(url: &str) -> Result<(), OpenPrivacySettingsError> {
    let status = std::process::Command::new("open")
        .arg(url)
        .status()
        .map_err(|error| {
            OpenPrivacySettingsError::command_failed(format!(
                "macOS Settings could not be opened: {error}"
            ))
        })?;
    if status.success() {
        return Ok(());
    }
    Err(OpenPrivacySettingsError::command_failed(format!(
        "macOS Settings exited with status {status}"
    )))
}

#[cfg(not(target_os = "macos"))]
fn open_settings_url(_url: &str) -> Result<(), OpenPrivacySettingsError> {
    Err(OpenPrivacySettingsError::unavailable())
}

#[cfg(test)]
mod tests {
    use super::PrivacySettingsPane;

    #[test]
    fn privacy_settings_panes_map_to_macos_privacy_urls() {
        let cases = [
            (
                PrivacySettingsPane::FullDiskAccess,
                "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles",
            ),
            (
                PrivacySettingsPane::Calendar,
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Calendars",
            ),
            (
                PrivacySettingsPane::Reminders,
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Reminders",
            ),
        ];

        for (pane, expected_url) in cases {
            assert_eq!(pane.settings_url(), expected_url);
        }
    }
}
