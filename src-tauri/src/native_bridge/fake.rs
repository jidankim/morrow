use std::{collections::BTreeMap, path::PathBuf};

use morrow_messages::{
    DiscoveredChat, MessagesDataSource, MessagesDiscoveryDataSource, MessagesDiscoveryReport,
    MessagesDiscoveryStatus, MessagesError, NativeBatch, NativeReadRequest,
};
use serde::Serialize;

use super::{
    keychain::{
        FakeMorrowTokenVault, KeychainBridgeError, TokenCommandReceipt, TokenLookupRequest,
        TokenReadResponse, TokenWriteRequest,
    },
    permissions::{map_permission_status, PermissionKind, PermissionState, PermissionStatus},
    public_chat_id::public_chat_id,
};

#[derive(Debug)]
pub struct FakeNativeBridge {
    permissions: BTreeMap<PermissionKind, PermissionState>,
    pub(crate) vault: FakeMorrowTokenVault,
    morrow_store_path: Option<PathBuf>,
    messages: Option<NativeBatch>,
    discovery_report: MessagesDiscoveryReport,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MessagesDiscoveryCommandReport {
    pub status: MessagesDiscoveryCommandStatus,
    pub chats: Vec<MessagesDiscoveryCommandChat>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MessagesDiscoveryCommandStatus {
    Ready,
    Empty,
    PermissionDenied,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MessagesDiscoveryCommandChat {
    pub chat_id: String,
    pub display_label: String,
    pub participant_count: u16,
    pub participant_ids: Vec<String>,
    pub latest_activity_timestamp: i64,
}

impl MessagesDiscoveryCommandReport {
    pub fn from_report(report: &MessagesDiscoveryReport) -> Self {
        Self {
            status: command_status(report.status()),
            chats: report
                .chats()
                .iter()
                .map(MessagesDiscoveryCommandChat::from_chat)
                .collect(),
        }
    }
}

impl MessagesDiscoveryCommandChat {
    fn from_chat(chat: &DiscoveredChat) -> Self {
        Self {
            chat_id: public_chat_id(chat.chat_guid()),
            display_label: chat.display_label().to_owned(),
            participant_count: chat.participant_count(),
            participant_ids: chat
                .participant_ids()
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
            latest_activity_timestamp: chat.latest_activity_timestamp().as_i64(),
        }
    }
}

const fn command_status(status: MessagesDiscoveryStatus) -> MessagesDiscoveryCommandStatus {
    match status {
        MessagesDiscoveryStatus::Ready => MessagesDiscoveryCommandStatus::Ready,
        MessagesDiscoveryStatus::Empty => MessagesDiscoveryCommandStatus::Empty,
        MessagesDiscoveryStatus::PermissionDenied => {
            MessagesDiscoveryCommandStatus::PermissionDenied
        }
        MessagesDiscoveryStatus::Unavailable => MessagesDiscoveryCommandStatus::Unavailable,
    }
}

impl Default for FakeNativeBridge {
    fn default() -> Self {
        Self {
            permissions: BTreeMap::new(),
            vault: FakeMorrowTokenVault::default(),
            morrow_store_path: None,
            messages: None,
            discovery_report: MessagesDiscoveryReport::permission_denied(),
        }
    }
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
            discovery_report: MessagesDiscoveryReport::permission_denied(),
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

    pub fn with_discovery_report(mut self, report: MessagesDiscoveryReport) -> Self {
        self.discovery_report = report;
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

impl MessagesDiscoveryDataSource for FakeNativeBridge {
    fn discover_chats(&self) -> Result<MessagesDiscoveryReport, MessagesError> {
        Ok(self.discovery_report.clone())
    }
}
