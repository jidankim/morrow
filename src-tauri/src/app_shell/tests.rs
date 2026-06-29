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
fn default_state_omits_absent_error_message_when_serialized() {
    let serialized = serde_json::to_value(AppShellState::default()).unwrap();

    assert!(serialized.get("errorMessage").is_none());
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
