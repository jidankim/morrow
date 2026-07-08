mod chat_id_cache;
mod production;

use std::path::Path;

use morrow_messages::{MessagesDiscoveryDataSource, MessagesDiscoveryReport, MessagesError};

use super::{
    delete_all, eventkit_cleanup, map_permission_status, production_scan, scan, scheduler,
    CodexExecRunner, DeleteMorrowDataError, DeleteMorrowDataRequest, FakeNativeBridge,
    KeychainBridgeError, MessagesPreviewCommandReport, MessagesPreviewRequest,
    MorrowDataDeleteReceipt, PermissionKind, PermissionState, PermissionStatus,
    ProductionScanCodexDependencies, ProposalReplayAdapter, ScanSelectedChatsError,
    ScanSelectedChatsRequest, ScanSelectedChatsResult, SyncSchedulerStateCommand,
    TokenCommandReceipt, TokenLookupRequest, TokenReadResponse, TokenWriteRequest,
};
pub(in crate::native_bridge) use chat_id_cache::SelectedChatIdResolutionCache;
use production::ProductionNativeBridge;

#[derive(Debug)]
pub struct NativeBridgeState {
    bridge: NativeBridgeBackend,
}

#[derive(Debug)]
enum NativeBridgeBackend {
    Production(ProductionNativeBridge),
    Fake(FakeNativeBridge),
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
                    &eventkit_cleanup::NoopProposedItemCleaner,
                )
            }
        }
    }

    pub(super) fn delete_morrow_data_at(
        &self,
        request: DeleteMorrowDataRequest,
        store_path: &Path,
    ) -> Result<MorrowDataDeleteReceipt, DeleteMorrowDataError> {
        match &self.bridge {
            NativeBridgeBackend::Production(bridge) => delete_all::delete_morrow_data_at(
                request,
                store_path,
                &bridge.token_vault,
                &eventkit_cleanup::EventKitProposedItemCleaner,
            ),
            NativeBridgeBackend::Fake(bridge) => delete_all::delete_morrow_data_at(
                request,
                store_path,
                &bridge.vault,
                &eventkit_cleanup::NoopProposedItemCleaner,
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
            NativeBridgeBackend::Production(bridge) => production_scan::scan_selected_chats_at(
                request,
                store_path,
                messages_db_path,
                &bridge.selected_chat_id_cache,
            ),
            NativeBridgeBackend::Fake(bridge) => {
                scan::scan_selected_chats_with_source(request, store_path, bridge)
            }
        }
    }

    pub fn scan_selected_chats_at_with_app_data_dir(
        &self,
        request: ScanSelectedChatsRequest,
        store_path: &Path,
        messages_db_path: &Path,
        app_data_dir: &Path,
    ) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError> {
        match &self.bridge {
            NativeBridgeBackend::Production(bridge) => {
                production_scan::scan_selected_chats_at_with_app_data_dir(
                    request,
                    store_path,
                    messages_db_path,
                    app_data_dir,
                    &bridge.selected_chat_id_cache,
                )
            }
            NativeBridgeBackend::Fake(bridge) => {
                scan::scan_selected_chats_with_source(request, store_path, bridge)
            }
        }
    }

    pub fn get_sync_scheduler_state_at(
        &self,
        store_path: &Path,
    ) -> Result<SyncSchedulerStateCommand, String> {
        scheduler::load_sync_scheduler_state_at(store_path)
    }

    pub fn set_sync_scheduler_state_at(
        &self,
        store_path: &Path,
        state: SyncSchedulerStateCommand,
    ) -> Result<SyncSchedulerStateCommand, String> {
        scheduler::save_sync_scheduler_state_at(store_path, state)
    }

    pub fn scan_selected_chats_at_with_codex_dependencies<R, A>(
        &self,
        request: ScanSelectedChatsRequest,
        store_path: &Path,
        messages_db_path: &Path,
        dependencies: ProductionScanCodexDependencies<'_, R, A>,
    ) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError>
    where
        R: CodexExecRunner,
        A: ProposalReplayAdapter,
    {
        match &self.bridge {
            NativeBridgeBackend::Production(bridge) => {
                production_scan::scan_selected_chats_at_with_codex_dependencies(
                    request,
                    store_path,
                    messages_db_path,
                    &bridge.selected_chat_id_cache,
                    dependencies,
                )
            }
            NativeBridgeBackend::Fake(bridge) => {
                scan::scan_selected_chats_with_source(request, store_path, bridge)
            }
        }
    }

    pub fn scan_selected_chats_at_with_codex_dependencies_and_app_data_dir<R, A>(
        &self,
        request: ScanSelectedChatsRequest,
        store_path: &Path,
        messages_db_path: &Path,
        app_data_dir: &Path,
        dependencies: ProductionScanCodexDependencies<'_, R, A>,
    ) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError>
    where
        R: CodexExecRunner,
        A: ProposalReplayAdapter,
    {
        match &self.bridge {
            NativeBridgeBackend::Production(bridge) => {
                production_scan::scan_selected_chats_at_with_codex_dependencies_and_app_data_dir(
                    request,
                    store_path,
                    messages_db_path,
                    app_data_dir,
                    &bridge.selected_chat_id_cache,
                    dependencies,
                )
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
            NativeBridgeBackend::Production(bridge) => bridge.discover_messages_chats_at(db_path),
            NativeBridgeBackend::Fake(bridge) => bridge.discover_chats(),
        }
    }

    pub fn load_messages_chat_previews_at(
        &self,
        db_path: &Path,
        request: &MessagesPreviewRequest,
    ) -> Result<MessagesPreviewCommandReport, MessagesError> {
        match &self.bridge {
            NativeBridgeBackend::Production(bridge) => {
                bridge.load_messages_chat_previews_at(db_path, request)
            }
            NativeBridgeBackend::Fake(bridge) => bridge.load_messages_chat_previews(request),
        }
    }
}
