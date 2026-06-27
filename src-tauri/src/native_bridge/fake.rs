use std::{collections::BTreeMap, path::PathBuf};

use morrow_messages::{MessagesDataSource, MessagesError, NativeBatch, NativeReadRequest};

use super::{
    keychain::{
        FakeMorrowTokenVault, KeychainBridgeError, TokenCommandReceipt, TokenLookupRequest,
        TokenReadResponse, TokenWriteRequest,
    },
    permissions::{map_permission_status, PermissionKind, PermissionState, PermissionStatus},
};

#[derive(Debug, Default)]
pub struct FakeNativeBridge {
    permissions: BTreeMap<PermissionKind, PermissionState>,
    pub(crate) vault: FakeMorrowTokenVault,
    morrow_store_path: Option<PathBuf>,
    messages: Option<NativeBatch>,
}

impl FakeNativeBridge {
    pub fn with_permission(kind: PermissionKind, state: PermissionState) -> Self {
        let mut bridge = Self::with_all_permissions(PermissionState::Unavailable);
        bridge.permissions.insert(kind, state);
        bridge
    }

    pub fn with_all_permissions(state: PermissionState) -> Self {
        let permissions = PermissionKind::ALL
            .into_iter()
            .map(|kind| (kind, state))
            .collect();
        Self {
            permissions,
            vault: FakeMorrowTokenVault::default(),
            morrow_store_path: None,
            messages: None,
        }
    }

    pub fn with_morrow_store_path(path: PathBuf) -> Self {
        let mut bridge = Self::with_all_permissions(PermissionState::Unavailable);
        bridge.morrow_store_path = Some(path);
        bridge
    }

    pub fn with_messages(mut self, messages: NativeBatch) -> Self {
        self.messages = Some(messages);
        self
    }

    pub fn query_permission_statuses(&self) -> Vec<PermissionStatus> {
        PermissionKind::ALL
            .into_iter()
            .map(|kind| {
                let state = match self.permissions.get(&kind).copied() {
                    Some(state) => state,
                    None => PermissionState::Unavailable,
                };
                map_permission_status(kind, state)
            })
            .collect()
    }

    pub fn create_morrow_token(
        &self,
        request: TokenWriteRequest,
    ) -> Result<TokenCommandReceipt, KeychainBridgeError> {
        self.vault.create(request)
    }

    pub fn read_morrow_token(
        &self,
        request: TokenLookupRequest,
    ) -> Result<TokenReadResponse, KeychainBridgeError> {
        self.vault.read(request)
    }

    pub fn delete_morrow_token(
        &self,
        request: TokenLookupRequest,
    ) -> Result<TokenCommandReceipt, KeychainBridgeError> {
        self.vault.delete(request)
    }

    pub fn morrow_store_path(&self) -> Option<&PathBuf> {
        self.morrow_store_path.as_ref()
    }
}

impl MessagesDataSource for FakeNativeBridge {
    fn read_recent(&self, _request: &NativeReadRequest) -> Result<NativeBatch, MessagesError> {
        match &self.messages {
            Some(messages) => Ok(messages.clone()),
            None => Err(MessagesError::PermissionDenied),
        }
    }
}
