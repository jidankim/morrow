use std::{collections::BTreeMap, fmt, sync::Mutex};

use super::{
    validate_lookup, validate_token, KeychainBridgeError, TokenCommandReceipt, TokenLookupRequest,
    TokenReadResponse, TokenStorageSurface, TokenWriteRequest,
};

#[derive(Default)]
pub struct FakeMorrowTokenVault {
    tokens: Mutex<BTreeMap<String, String>>,
}

impl fmt::Debug for FakeMorrowTokenVault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let token_count = self.tokens.lock().map_or(0, |tokens| tokens.len());
        formatter
            .debug_struct("FakeMorrowTokenVault")
            .field("token_count", &token_count)
            .field("tokens", &"<redacted>")
            .finish()
    }
}

impl FakeMorrowTokenVault {
    pub fn create(
        &self,
        request: TokenWriteRequest,
    ) -> Result<TokenCommandReceipt, KeychainBridgeError> {
        validate_lookup(&request.lookup())?;
        validate_token(&request.token)?;
        let mut tokens = self.tokens.lock().map_err(|_| {
            KeychainBridgeError::storage_unavailable("fake Morrow token vault lock is poisoned")
        })?;
        tokens.insert(request.token_kind, request.token);
        Ok(TokenCommandReceipt {
            storage_surface: TokenStorageSurface::KeychainBridge,
            stored: true,
            deleted: false,
        })
    }

    pub fn read(
        &self,
        request: TokenLookupRequest,
    ) -> Result<TokenReadResponse, KeychainBridgeError> {
        validate_lookup(&request)?;
        let token = self
            .tokens
            .lock()
            .map_err(|_| {
                KeychainBridgeError::storage_unavailable("fake Morrow token vault lock is poisoned")
            })?
            .get(&request.token_kind)
            .cloned();
        Ok(TokenReadResponse {
            storage_surface: TokenStorageSurface::KeychainBridge,
            present: token.is_some(),
            token,
        })
    }

    pub fn delete(
        &self,
        request: TokenLookupRequest,
    ) -> Result<TokenCommandReceipt, KeychainBridgeError> {
        validate_lookup(&request)?;
        let mut tokens = self.tokens.lock().map_err(|_| {
            KeychainBridgeError::storage_unavailable("fake Morrow token vault lock is poisoned")
        })?;
        let deleted = tokens.remove(&request.token_kind).is_some();
        Ok(TokenCommandReceipt {
            storage_surface: TokenStorageSurface::KeychainBridge,
            stored: false,
            deleted,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_bridge::{MORROW_KEYCHAIN_SERVICE, MORROW_PROVIDER_TOKEN_KIND};

    #[test]
    fn fake_vault_debug_redacts_stored_tokens() {
        let vault = FakeMorrowTokenVault::default();
        vault
            .create(TokenWriteRequest::new(
                MORROW_KEYCHAIN_SERVICE,
                MORROW_PROVIDER_TOKEN_KIND,
                "sk-fake-vault-canary",
            ))
            .expect("fake write");

        let debug = format!("{vault:?}");

        assert!(debug.contains("token_count"));
        assert!(!debug.contains("sk-fake-vault-canary"), "{debug}");
        assert!(!debug.contains(MORROW_PROVIDER_TOKEN_KIND), "{debug}");
    }
}
