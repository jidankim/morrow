use super::{menu_model, sync_now_event_allowed, AppMode, AppShellState, AutomaticSyncStatusLabel};

#[test]
fn menu_model_reports_setup_needed_by_default() {
    let state = AppShellState::default();

    let menu = menu_model(&state);

    assert_eq!(menu.status_kind, "setup-needed");
    assert_eq!(menu.status_label, "Setup needed");
    assert_eq!(menu.pause_resume_label, "Disable Sync Now");
    assert!(!menu.sync_now_enabled);
    assert!(!sync_now_event_allowed(&state));
    assert_eq!(menu.settings_label, "Settings");
    assert_eq!(menu.open_calendar_label, "Open Calendar");
    assert_eq!(menu.open_reminders_label, "Open Reminders");
    assert_eq!(menu.quit_label, "Quit Morrow");
    assert_eq!(menu.pending_proposal_label, "0");
    assert_eq!(menu.automatic_sync_label, "Automatic Sync: Off");
    assert!(!menu.automatic_sync_enabled);
    assert_eq!(menu.automatic_sync_detail, "Automatic sync is off.");
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
    assert_eq!(menu.status_label, "Ready");
    assert_eq!(
        menu.detail,
        "Ready. Use Sync Now to reconcile calendars and scan selected chats."
    );
    assert!(menu.sync_now_enabled);
    assert!(sync_now_event_allowed(&state));
    assert_eq!(menu.pause_resume_label, "Disable Sync Now");
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

    assert_eq!(menu.status_label, "Sync Now disabled");
    assert_eq!(menu.status_kind, "paused");
    assert_eq!(menu.detail, "Sync Now is disabled on this Mac.");
    assert!(!menu.sync_now_enabled);
    assert!(!sync_now_event_allowed(&state));
    assert_eq!(menu.pause_resume_label, "Enable Sync Now");
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
    assert_eq!(menu.pause_resume_label, "Disable Sync Now");
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
    assert_eq!(menu.pause_resume_label, "Disable Sync Now");
    assert!(!menu.sync_now_enabled);
    assert!(!sync_now_event_allowed(&state));
}

#[test]
fn menu_model_reports_automatic_sync_on_label() {
    let state = AppShellState {
        automatic_sync_enabled: true,
        automatic_sync_status_label: AutomaticSyncStatusLabel::On,
        automatic_sync_detail: "Next run at 09:30.".to_owned(),
        ..AppShellState::default()
    };

    let menu = menu_model(&state);

    assert_eq!(menu.automatic_sync_label, "Automatic Sync: On");
    assert!(menu.automatic_sync_enabled);
    assert_eq!(menu.automatic_sync_detail, "Next run at 09:30.");
}

#[test]
fn menu_model_reports_automatic_sync_cooling_down_label() {
    let state = AppShellState {
        automatic_sync_enabled: true,
        automatic_sync_status_label: AutomaticSyncStatusLabel::CoolingDown,
        automatic_sync_detail: "Retrying after a transient failure.".to_owned(),
        ..AppShellState::default()
    };

    let menu = menu_model(&state);

    assert_eq!(menu.automatic_sync_label, "Automatic Sync: Cooling Down");
    assert!(menu.automatic_sync_enabled);
    assert_eq!(
        menu.automatic_sync_detail,
        "Retrying after a transient failure."
    );
}

#[test]
fn menu_model_reports_automatic_sync_needs_action_label() {
    let state = AppShellState {
        automatic_sync_enabled: true,
        automatic_sync_status_label: AutomaticSyncStatusLabel::NeedsAction,
        automatic_sync_detail: "Choose at least one chat before automatic sync can run.".to_owned(),
        ..AppShellState::default()
    };

    let menu = menu_model(&state);

    assert_eq!(menu.automatic_sync_label, "Automatic Sync: Needs Action");
    assert!(menu.automatic_sync_enabled);
    assert_eq!(
        menu.automatic_sync_detail,
        "Choose at least one chat before automatic sync can run."
    );
}
