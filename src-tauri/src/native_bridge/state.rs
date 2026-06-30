use std::path::Path;

use morrow_messages::{MessagesDiscoveryDataSource, MessagesDiscoveryReport, MessagesError};

use super::{
    codex_auth, delete_all, eventkit_cleanup, eventkit_proposal, map_permission_status,
    messages_sqlite, scan, CodexExecRunner, CodexProvider, CodexProviderAuthReadiness,
    DeleteMorrowDataError, DeleteMorrowDataRequest, FakeNativeBridge, KeychainBridgeError,
    MessagesPreviewCommandReport, MessagesPreviewRequest, MorrowDataDeleteReceipt,
    MorrowTokenVault, PermissionKind, PermissionState, PermissionStatus, ProcessCodexExecRunner,
    ProposalReplayAdapter, ScanSelectedChatsError, ScanSelectedChatsRequest,
    ScanSelectedChatsResult, TokenCommandReceipt, TokenLookupRequest, TokenReadResponse,
    TokenWriteRequest,
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

pub struct ProductionScanCodexDependencies<'a, R, A> {
    pub auth_readiness: CodexProviderAuthReadiness,
    pub codex_runner: &'a R,
    pub proposal_adapter: &'a A,
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
            NativeBridgeBackend::Production(_) => {
                let proposal_adapter = eventkit_proposal::EventKitProposalBridge;
                let codex_runner = ProcessCodexExecRunner;
                self.scan_selected_chats_at_with_codex_dependencies(
                    request,
                    store_path,
                    messages_db_path,
                    ProductionScanCodexDependencies {
                        auth_readiness: codex_auth::probe_codex_provider_auth(),
                        codex_runner: &codex_runner,
                        proposal_adapter: &proposal_adapter,
                    },
                )
            }
            NativeBridgeBackend::Fake(bridge) => {
                scan::scan_selected_chats_with_source(request, store_path, bridge)
            }
        }
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
            NativeBridgeBackend::Production(_) => scan_selected_chats_at_with_provider_mode(
                request,
                store_path,
                messages_db_path,
                dependencies,
            ),
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
                messages_sqlite::MessagesSqliteAdapter::new(db_path.to_path_buf()).discover_chats()
            }
            NativeBridgeBackend::Fake(bridge) => bridge.discover_chats(),
        }
    }

    pub fn load_messages_chat_previews_at(
        &self,
        db_path: &Path,
        request: &MessagesPreviewRequest,
    ) -> Result<MessagesPreviewCommandReport, MessagesError> {
        match &self.bridge {
            NativeBridgeBackend::Production(_) => {
                messages_sqlite::MessagesSqliteAdapter::new(db_path.to_path_buf())
                    .load_messages_chat_previews(request)
            }
            NativeBridgeBackend::Fake(bridge) => bridge.load_messages_chat_previews(request),
        }
    }
}

fn scan_selected_chats_at_with_provider_mode<R, A>(
    request: ScanSelectedChatsRequest,
    store_path: &Path,
    messages_db_path: &Path,
    dependencies: ProductionScanCodexDependencies<'_, R, A>,
) -> Result<ScanSelectedChatsResult, ScanSelectedChatsError>
where
    R: CodexExecRunner,
    A: ProposalReplayAdapter,
{
    if dependencies.auth_readiness.ready {
        let provider = CodexProvider::new(dependencies.codex_runner);
        scan::scan_selected_chats_at_with_dependencies(
            request,
            store_path,
            messages_db_path,
            &provider,
            dependencies.proposal_adapter,
        )
    } else {
        scan::scan_selected_chats_at_with_unavailable_provider(
            request,
            store_path,
            messages_db_path,
            dependencies.proposal_adapter,
        )
    }
}
