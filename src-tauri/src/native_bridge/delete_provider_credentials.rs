use super::{
    delete_all_protocol::ProviderCredentialDeleteReceipt, keychain::FakeMorrowTokenVault,
    KeychainBridgeError, MorrowTokenVault, TokenLookupRequest, MORROW_KEYCHAIN_SERVICE,
    MORROW_PROVIDER_TOKEN_KIND, MORROW_TOKEN_KIND,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderOAuthDeleteReceipt {
    pub(crate) deleted: bool,
    pub(crate) failed: bool,
    pub(crate) error: Option<String>,
}

pub(crate) trait MorrowTokenDeleter {
    fn delete_morrow_token(&self, request: TokenLookupRequest)
        -> Result<bool, KeychainBridgeError>;
}

impl MorrowTokenDeleter for MorrowTokenVault {
    fn delete_morrow_token(
        &self,
        request: TokenLookupRequest,
    ) -> Result<bool, KeychainBridgeError> {
        self.delete(request).map(|receipt| receipt.deleted)
    }
}

impl MorrowTokenDeleter for FakeMorrowTokenVault {
    fn delete_morrow_token(
        &self,
        request: TokenLookupRequest,
    ) -> Result<bool, KeychainBridgeError> {
        self.delete(request).map(|receipt| receipt.deleted)
    }
}

pub(crate) fn provider_credential_delete_receipts(
    requested: bool,
    token_vault: &impl MorrowTokenDeleter,
) -> Vec<ProviderCredentialDeleteReceipt> {
    provider_token_kinds()
        .into_iter()
        .map(|token_kind| provider_credential_delete_receipt(requested, token_kind, token_vault))
        .collect()
}

fn provider_credential_delete_receipt(
    requested: bool,
    token_kind: &str,
    token_vault: &impl MorrowTokenDeleter,
) -> ProviderCredentialDeleteReceipt {
    if !requested {
        return ProviderCredentialDeleteReceipt {
            token_kind: token_kind.to_owned(),
            delete_requested: false,
            deleted: false,
            failed: false,
            error: None,
        };
    }

    match token_vault
        .delete_morrow_token(TokenLookupRequest::new(MORROW_KEYCHAIN_SERVICE, token_kind))
    {
        Ok(deleted) => ProviderCredentialDeleteReceipt {
            token_kind: token_kind.to_owned(),
            delete_requested: true,
            deleted,
            failed: false,
            error: None,
        },
        Err(error) => ProviderCredentialDeleteReceipt {
            token_kind: token_kind.to_owned(),
            delete_requested: true,
            deleted: false,
            failed: true,
            error: Some(error.to_string()),
        },
    }
}

pub(crate) fn provider_oauth_delete_receipt(
    receipts: &[ProviderCredentialDeleteReceipt],
) -> ProviderOAuthDeleteReceipt {
    let deleted = receipts.iter().any(|receipt| receipt.deleted);
    let failed = receipts.iter().any(|receipt| receipt.failed);
    let error_messages = receipts
        .iter()
        .filter_map(|receipt| {
            receipt
                .error
                .as_ref()
                .map(|error| format!("{}: {error}", receipt.token_kind))
        })
        .collect::<Vec<_>>();
    ProviderOAuthDeleteReceipt {
        deleted,
        failed,
        error: if error_messages.is_empty() {
            None
        } else {
            Some(error_messages.join("; "))
        },
    }
}

const fn provider_token_kinds() -> [&'static str; 2] {
    [MORROW_TOKEN_KIND, MORROW_PROVIDER_TOKEN_KIND]
}
