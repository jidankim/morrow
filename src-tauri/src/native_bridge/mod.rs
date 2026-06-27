mod delete_all;
mod delete_all_protocol;
mod eventkit_cleanup;
mod fake;
mod keychain;
mod permissions;
mod scan;

use eventkit_cleanup::{EventKitProposedItemCleaner, NoopProposedItemCleaner};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

pub use delete_all_protocol::{
    DeleteCleanupAction, DeleteCleanupPlan, DeleteMorrowDataError, DeleteMorrowDataRequest,
    MorrowDataDeleteReceipt, MorrowDataStorageSurface,
};
pub use fake::FakeNativeBridge;
pub use keychain::{
    KeychainBridgeError, KeychainErrorCode, MorrowTokenVault, TokenCommandReceipt,
    TokenLookupRequest, TokenReadResponse, TokenStorageSurface, TokenWriteRequest,
    MORROW_KEYCHAIN_SERVICE, MORROW_TOKEN_KIND,
};
pub use permissions::{
    map_permission_status, OpenPrivacySettingsError, OpenPrivacySettingsReceipt,
    OpenPrivacySettingsRequest, PermissionKind, PermissionOutcome, PermissionState,
    PermissionStatus, PrivacySettingsPane,
};
pub use scan::{
    CapPolicyRequest, ScanSelectedChatsError, ScanSelectedChatsRequest, ScanSelectedChatsResult,
};

#[derive(Debug)]
pub struct NativeBridgeState {
    bridge: NativeBridgeBackend,
}

#[derive(Debug)]
enum NativeBridgeBackend {
    Production(ProductionNativeBridge),
    Fake(FakeNativeBridge),
}

#[derive(Debug, Default)]
struct ProductionNativeBridge {
    token_vault: MorrowTokenVault,
}

impl Default for NativeBridgeState {
    fn default() -> Self {
        Self {
            bridge: NativeBridgeBackend::Production(ProductionNativeBridge::default()),
        }
    }
}

impl NativeBridgeState {
    pub fn with_bridge(bridge: FakeNativeBridge) -> Self {
        Self {
            bridge: NativeBridgeBackend::Fake(bridge),
        }
    }

    pub fn query_permission_statuses(&self) -> Vec<PermissionStatus> {
        match &self.bridge {
            NativeBridgeBackend::Production(_) => PermissionKind::ALL
                .into_iter()
                .map(|kind| map_permission_status(kind, PermissionState::Unavailable))
                .collect(),
            NativeBridgeBackend::Fake(bridge) => bridge.query_permission_statuses(),
        }
    }

    pub fn create_morrow_token(
        &self,
        request: TokenWriteRequest,
    ) -> Result<TokenCommandReceipt, KeychainBridgeError> {
        match &self.bridge {
            NativeBridgeBackend::Production(bridge) => bridge.token_vault.create(request),
            NativeBridgeBackend::Fake(bridge) => bridge.create_morrow_token(request),
        }
    }

    pub fn read_morrow_token(
        &self,
        request: TokenLookupRequest,
    ) -> Result<TokenReadResponse, KeychainBridgeError> {
        match &self.bridge {
            NativeBridgeBackend::Production(bridge) => bridge.token_vault.read(request),
            NativeBridgeBackend::Fake(bridge) => bridge.read_morrow_token(request),
        }
    }

    pub fn delete_morrow_token(
        &self,
        request: TokenLookupRequest,
    ) -> Result<TokenCommandReceipt, KeychainBridgeError> {
        match &self.bridge {
            NativeBridgeBackend::Production(bridge) => bridge.token_vault.delete(request),
            NativeBridgeBackend::Fake(bridge) => bridge.delete_morrow_token(request),
        }
    }

    pub fn delete_morrow_data(
        &self,
        request: DeleteMorrowDataRequest,
    ) -> Result<MorrowDataDeleteReceipt, DeleteMorrowDataError> {
        match &self.bridge {
            NativeBridgeBackend::Production(_) => Err(DeleteMorrowDataError::storage_unavailable(
                "production Morrow store deletion requires an app data path",
            )),
            NativeBridgeBackend::Fake(bridge) => {
                let store_path = bridge.morrow_store_path().ok_or_else(|| {
                    DeleteMorrowDataError::storage_unavailable(
                        "fake Morrow store deletion requires a configured store path",
                    )
                })?;
                delete_all::delete_morrow_data_at(
                    request,
                    store_path,
                    &bridge.vault,
                    &NoopProposedItemCleaner,
                )
            }
        }
    }

    fn delete_morrow_data_at(
        &self,
        request: DeleteMorrowDataRequest,
        store_path: &std::path::Path,
    ) -> Result<MorrowDataDeleteReceipt, DeleteMorrowDataError> {
        match &self.bridge {
            NativeBridgeBackend::Production(bridge) => delete_all::delete_morrow_data_at(
                request,
                store_path,
                &bridge.token_vault,
                &EventKitProposedItemCleaner,
            ),
            NativeBridgeBackend::Fake(bridge) => delete_all::delete_morrow_data_at(
                request,
                store_path,
                &bridge.vault,
                &NoopProposedItemCleaner,
            ),
        }
    }

    pub fn scan_selected_chats_at(
        &self,
        request: ScanSelectedChatsRequest,
        store_path: &std::path::Path,
    ) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError> {
        match &self.bridge {
            NativeBridgeBackend::Production(_) => scan::scan_selected_chats_at(request, store_path),
            NativeBridgeBackend::Fake(bridge) => {
                scan::scan_selected_chats_with_source(request, store_path, bridge)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CrashLogRequest {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CrashLogReceipt {
    pub stored: bool,
}

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
    let store_path = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("morrow.sqlite");
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
    let store_path = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("morrow.sqlite");
    state
        .scan_selected_chats_at(request, &store_path)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn reconcile_now(app: AppHandle) -> Result<(), String> {
    let store_path = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("morrow.sqlite");
    morrow_storage::Store::open(&store_path)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn record_crash_log(request: CrashLogRequest) -> Result<CrashLogReceipt, String> {
    let _request = request;
    Ok(CrashLogReceipt { stored: false })
}
