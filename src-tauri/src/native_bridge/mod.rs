mod codex_auth;
mod codex_provider;
mod crash_log;
mod delete_all;
mod delete_all_protocol;
mod delete_provider_credentials;
mod eventkit_cleanup;
pub mod eventkit_proposal;
mod fake;
mod keychain;
pub mod messages_sqlite;
mod openai_provider;
mod paths;
mod permissions;
mod preview;
mod production_scan;
mod provider_contract;
mod public_chat_id;
mod runtime_identity;
mod scan;
mod scan_privacy;
mod scheduler;
mod state;
mod store_probe;

use paths::{app_data_dir, messages_database_path, morrow_store_path};
use tauri::{AppHandle, State};

pub use codex_auth::{
    probe_codex_provider_auth, probe_codex_provider_auth_with_runner, CodexAuthCommandOutput,
    CodexAuthCommandRunner, CodexAuthProbeOptions, CodexAuthStatus, CodexLoginStatusRun,
    CodexProviderAuthReadiness, ProcessCodexAuthCommandRunner,
};
pub use codex_provider::{
    CodexCommandOutput, CodexExecRequest, CodexExecRun, CodexExecRunner, CodexProvider,
    CodexProviderError, ProcessCodexExecRunner,
};
pub use crash_log::{
    __cmd__record_crash_log, __tauri_command_name_record_crash_log, record_crash_log,
    CrashLogReceipt, CrashLogRequest,
};
pub use delete_all_protocol::{
    DeleteCleanupAction, DeleteCleanupPlan, DeleteMorrowDataError, DeleteMorrowDataRequest,
    MorrowDataDeleteReceipt, MorrowDataStorageSurface,
};
pub use fake::{FakeNativeBridge, MessagesDiscoveryCommandReport};
pub use keychain::{
    KeychainBridgeError, KeychainErrorCode, MorrowTokenVault, TokenCommandReceipt,
    TokenLookupRequest, TokenReadResponse, TokenStorageSurface, TokenWriteRequest,
    MORROW_KEYCHAIN_SERVICE, MORROW_PROVIDER_TOKEN_KIND, MORROW_TOKEN_KIND,
};
pub use openai_provider::{
    OpenAiHttpRequest, OpenAiHttpResponse, OpenAiProvider, OpenAiProviderError, OpenAiTransport,
    ReqwestOpenAiTransport, OPENAI_MODEL, OPENAI_RESPONSES_URL, OPENAI_TIMEOUT_MS,
};
pub use permissions::{
    map_permission_status, OpenPrivacySettingsError, OpenPrivacySettingsReceipt,
    OpenPrivacySettingsRequest, PermissionKind, PermissionOutcome, PermissionState,
    PermissionStatus, PrivacySettingsPane,
};
pub use preview::{
    MessagesPreviewCommandChat, MessagesPreviewCommandReport, MessagesPreviewRequest,
};
pub use production_scan::ProductionScanCodexDependencies;
pub use runtime_identity::{
    __cmd__get_runtime_identity, __tauri_command_name_get_runtime_identity, get_runtime_identity,
    runtime_identity_from_executable_path, RuntimeIdentity, RuntimeKind,
};
pub use scan::{
    scan_selected_chats_with_dependencies, CalendarProposalReceipt, CapPolicyRequest,
    ProposalReplayAdapter, ScanSelectedChatsDependencies, ScanSelectedChatsError,
    ScanSelectedChatsRequest, ScanSelectedChatsResult,
};
pub use scheduler::{
    SyncSchedulerLastResultCommand, SyncSchedulerStateCommand, SyncSchedulerStatusCommand,
};
pub use state::NativeBridgeState;

const MESSAGES_PREVIEW_UNAVAILABLE_ERROR: &str =
    "Messages previews are unavailable. Grant Full Disk Access or try again.";

#[tauri::command]
pub fn get_native_permission_statuses(
    state: State<'_, NativeBridgeState>,
) -> Result<Vec<PermissionStatus>, String> {
    Ok(state.query_permission_statuses())
}

#[tauri::command]
pub fn open_privacy_settings(
    request: OpenPrivacySettingsRequest,
) -> Result<OpenPrivacySettingsReceipt, String> {
    permissions::open_privacy_settings(request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn store_morrow_token(
    state: State<'_, NativeBridgeState>,
    request: TokenWriteRequest,
) -> Result<TokenCommandReceipt, String> {
    state
        .create_morrow_token(request)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn read_morrow_token(
    state: State<'_, NativeBridgeState>,
    request: TokenLookupRequest,
) -> Result<TokenReadResponse, String> {
    state
        .read_morrow_token(request)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_morrow_token(
    state: State<'_, NativeBridgeState>,
    request: TokenLookupRequest,
) -> Result<TokenCommandReceipt, String> {
    state
        .delete_morrow_token(request)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_morrow_data(
    app: AppHandle,
    state: State<'_, NativeBridgeState>,
    request: DeleteMorrowDataRequest,
) -> Result<MorrowDataDeleteReceipt, String> {
    let store_path = morrow_store_path(&app)?;
    state
        .delete_morrow_data_at(request, &store_path)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn scan_selected_chats(
    app: AppHandle,
    state: State<'_, NativeBridgeState>,
    request: ScanSelectedChatsRequest,
) -> Result<ScanSelectedChatsResult, String> {
    request
        .validate_local_diagnostics_retention()
        .map_err(|error| error.to_string())?;
    let store_path = morrow_store_path(&app)?;
    let app_data_dir = app_data_dir(&app)?;
    let messages_db_path = messages_database_path()?;
    state
        .scan_selected_chats_at_with_app_data_dir(
            request,
            &store_path,
            &messages_db_path,
            &app_data_dir,
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_sync_scheduler_state(
    app: AppHandle,
    state: State<'_, NativeBridgeState>,
) -> Result<SyncSchedulerStateCommand, String> {
    let store_path = morrow_store_path(&app)?;
    state.get_sync_scheduler_state_at(&store_path)
}

#[tauri::command]
pub fn set_sync_scheduler_state(
    app: AppHandle,
    bridge_state: State<'_, NativeBridgeState>,
    state: SyncSchedulerStateCommand,
) -> Result<SyncSchedulerStateCommand, String> {
    let store_path = morrow_store_path(&app)?;
    bridge_state.set_sync_scheduler_state_at(&store_path, state)
}

#[tauri::command]
pub fn discover_messages_chats(
    state: State<'_, NativeBridgeState>,
) -> Result<MessagesDiscoveryCommandReport, String> {
    let db_path = messages_database_path()?;
    let report = state
        .discover_messages_chats_at(&db_path)
        .map_err(|_error| {
            "Messages discovery is unavailable. Grant Full Disk Access or try again.".to_owned()
        })?;
    Ok(MessagesDiscoveryCommandReport::from_report(&report))
}

#[tauri::command]
pub fn load_messages_chat_previews(
    state: State<'_, NativeBridgeState>,
    request: MessagesPreviewRequest,
) -> Result<MessagesPreviewCommandReport, String> {
    let db_path =
        messages_database_path().map_err(|_error| MESSAGES_PREVIEW_UNAVAILABLE_ERROR.to_owned())?;
    state
        .load_messages_chat_previews_at(&db_path, &request)
        .map_err(|_error| MESSAGES_PREVIEW_UNAVAILABLE_ERROR.to_owned())
}

#[tauri::command]
pub fn check_provider_auth() -> CodexProviderAuthReadiness {
    codex_auth::probe_codex_provider_auth()
}

#[tauri::command]
pub fn reconcile_now(app: AppHandle) -> Result<(), String> {
    let store_path = morrow_store_path(&app)?;
    store_probe::reconcile_now_at(&store_path).map_err(|error| error.to_string())
}

#[cfg(test)]
mod runtime_identity_tests {
    use super::{get_runtime_identity, runtime_identity_from_executable_path, RuntimeKind};

    #[test]
    fn runtime_identity_reports_app_bundle_target_when_executable_is_inside_app_bundle() {
        // Given
        let executable_path = "/Applications/Morrow.app/Contents/MacOS/morrow";

        // When
        let identity = runtime_identity_from_executable_path(executable_path);

        // Then
        assert_eq!(identity.display_name, "Morrow");
        assert_eq!(identity.bundle_identifier, "dev.morrow.desktop");
        assert_eq!(identity.executable_path, executable_path);
        assert_eq!(identity.settings_target_path, "/Applications/Morrow.app");
        assert_eq!(identity.runtime_kind, RuntimeKind::AppBundle);
    }

    #[test]
    fn runtime_identity_reports_binary_target_when_executable_is_direct_binary() {
        // Given
        let executable_path = "/Users/example/workspace/morrow/src-tauri/target/debug/morrow";

        // When
        let identity = runtime_identity_from_executable_path(executable_path);

        // Then
        assert_eq!(identity.executable_path, executable_path);
        assert_eq!(identity.settings_target_path, executable_path);
        assert_eq!(identity.runtime_kind, RuntimeKind::Binary);
    }

    #[test]
    fn runtime_identity_keeps_non_app_paths_as_binary_targets() {
        // Given
        let executable_path = "/tmp/not-an-app/Contents/MacOS/morrow";

        // When
        let identity = runtime_identity_from_executable_path(executable_path);

        // Then
        assert_eq!(identity.settings_target_path, executable_path);
        assert_eq!(identity.runtime_kind, RuntimeKind::Binary);
    }

    #[test]
    fn runtime_identity_serializes_to_camel_case_contract() -> Result<(), String> {
        // Given
        let executable_path = "/Applications/Morrow.app/Contents/MacOS/morrow";
        let identity = runtime_identity_from_executable_path(executable_path);

        // When
        let json = serde_json::to_value(identity).map_err(|error| error.to_string())?;

        // Then
        assert_eq!(json["displayName"], "Morrow");
        assert_eq!(json["bundleIdentifier"], "dev.morrow.desktop");
        assert_eq!(json["executablePath"], executable_path);
        assert_eq!(json["settingsTargetPath"], "/Applications/Morrow.app");
        assert_eq!(json["runtimeKind"], "appBundle");
        assert_eq!(json.as_object().map(|object| object.len()), Some(5));
        Ok(())
    }

    #[test]
    fn runtime_identity_command_uses_current_executable() -> Result<(), String> {
        // Given / When
        let identity = get_runtime_identity()?;

        // Then
        assert_eq!(identity.display_name, "Morrow");
        assert_eq!(identity.bundle_identifier, "dev.morrow.desktop");
        assert!(!identity.executable_path.is_empty());
        assert!(!identity.settings_target_path.is_empty());
        Ok(())
    }
}
