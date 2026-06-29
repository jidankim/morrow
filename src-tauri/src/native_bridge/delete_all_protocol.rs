use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

use super::KeychainBridgeError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteMorrowDataRequest {
    pub confirmation: String,
    pub cleanup_proposed_items: bool,
    pub delete_empty_proposal_containers: bool,
    #[serde(rename = "revokeProviderOAuth", alias = "revokeProviderOauth")]
    pub revoke_provider_oauth: bool,
}

impl DeleteMorrowDataRequest {
    pub fn new(
        confirmation: &str,
        cleanup_proposed_items: bool,
        delete_empty_proposal_containers: bool,
        revoke_provider_oauth: bool,
    ) -> Self {
        Self {
            confirmation: confirmation.to_owned(),
            cleanup_proposed_items,
            delete_empty_proposal_containers,
            revoke_provider_oauth,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MorrowDataStorageSurface {
    MorrowStore,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DeleteCleanupAction {
    Completed,
    AdapterDeferred,
    SkippedByUser,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteCleanupPlan {
    pub proposed_items: DeleteCleanupAction,
    pub empty_proposal_containers: DeleteCleanupAction,
    pub proposed_calendar_items_deleted: u64,
    pub proposed_reminder_items_deleted: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCredentialDeleteReceipt {
    pub token_kind: String,
    pub delete_requested: bool,
    pub deleted: bool,
    pub failed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MorrowDataDeleteReceipt {
    pub storage_surface: MorrowDataStorageSurface,
    pub database_deleted: bool,
    pub approved_external_items_deleted: bool,
    #[serde(
        alias = "providerOAuthDeleteRequested",
        alias = "providerOauthDeleteRequested"
    )]
    pub provider_credentials_delete_requested: bool,
    #[serde(alias = "providerOAuthDeleted", alias = "providerOauthDeleted")]
    pub provider_credentials_deleted: bool,
    #[serde(
        alias = "providerOAuthDeleteFailed",
        alias = "providerOauthDeleteFailed"
    )]
    pub provider_credentials_delete_failed: bool,
    #[serde(
        alias = "providerOAuthDeleteError",
        alias = "providerOauthDeleteError",
        skip_serializing_if = "Option::is_none"
    )]
    pub provider_credentials_delete_error: Option<String>,
    pub provider_credential_deletes: Vec<ProviderCredentialDeleteReceipt>,
    pub cleanup_plan: DeleteCleanupPlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteMorrowDataError {
    message: String,
}

impl DeleteMorrowDataError {
    pub fn storage_unavailable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for DeleteMorrowDataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for DeleteMorrowDataError {}

impl From<morrow_storage::StorageError> for DeleteMorrowDataError {
    fn from(error: morrow_storage::StorageError) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}

impl From<KeychainBridgeError> for DeleteMorrowDataError {
    fn from(error: KeychainBridgeError) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}
