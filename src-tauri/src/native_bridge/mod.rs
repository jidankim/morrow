mod crash_log;
mod delete_all;
mod delete_all_protocol;
mod eventkit_cleanup;
mod fake;
mod keychain;
pub mod messages_sqlite;
mod permissions;
mod public_chat_id;
mod scan;

use std::path::{Path, PathBuf};

use eventkit_cleanup::{EventKitProposedItemCleaner, NoopProposedItemCleaner};
use messages_sqlite::MessagesSqliteAdapter;
use morrow_messages::{MessagesDiscoveryDataSource, MessagesDiscoveryReport, MessagesError};
use tauri::{AppHandle, Manager, State};

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
        store_path: &Path,
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
        store_path: &Path,
        messages_db_path: &Path,
    ) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError> {
        match &self.bridge {
            NativeBridgeBackend::Production(_) => {
                scan::scan_selected_chats_at(request, store_path, messages_db_path)
            }
            NativeBridgeBackend::Fake(bridge) => {
                scan::scan_selected_chats_with_source(request, store_path, bridge)
            }
        }
    }

    pub fn discover_messages_chats_at(
        &self,
        db_path: &Path,
    ) -> Result<MessagesDiscoveryReport, MessagesError> {
        match &self.bridge {
            NativeBridgeBackend::Production(_) => {
                MessagesSqliteAdapter::new(db_path.to_path_buf()).discover_chats()
            }
            NativeBridgeBackend::Fake(bridge) => bridge.discover_chats(),
        }
    }
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

fn messages_database_path() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join("Library/Messages/chat.db"))
        .ok_or_else(|| "Messages discovery is unavailable on this system.".to_owned())
}

fn morrow_store_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.join("morrow.sqlite"))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn reconcile_now(app: AppHandle) -> Result<(), String> {
    let store_path = morrow_store_path(&app)?;
    morrow_storage::Store::open(&store_path)
        .map(|_| ())
        .map_err(|error| error.to_string())
}
