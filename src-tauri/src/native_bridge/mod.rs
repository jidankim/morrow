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
mod provider_contract;
mod public_chat_id;
mod scan;
mod scan_privacy;
mod state;
mod store_probe;

use paths::{messages_database_path, morrow_store_path};
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
pub use scan::{
    scan_selected_chats_with_dependencies, CalendarProposalReceipt, CapPolicyRequest,
    ProposalReplayAdapter, ScanSelectedChatsDependencies, ScanSelectedChatsError,
    ScanSelectedChatsRequest, ScanSelectedChatsResult,
};
pub use state::{NativeBridgeState, ProductionScanCodexDependencies};

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
    let store_path = morrow_store_path(&app)?;
    let messages_db_path = messages_database_path()?;
    state
        .scan_selected_chats_at(request, &store_path, &messages_db_path)
        .map_err(|error| error.to_string())
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
pub fn check_provider_auth() -> CodexProviderAuthReadiness {
    codex_auth::probe_codex_provider_auth()
}

#[tauri::command]
pub fn reconcile_now(app: AppHandle) -> Result<(), String> {
    let store_path = morrow_store_path(&app)?;
    store_probe::reconcile_now_at(&store_path).map_err(|error| error.to_string())
}
