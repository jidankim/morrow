use std::{error::Error, fmt, sync::Mutex};

use serde::{Deserialize, Serialize};

#[cfg(target_os = "macos")]
mod macos_keychain;

pub const MORROW_KEYCHAIN_SERVICE: &str = "com.morrow.desktop.token";
pub const MORROW_TOKEN_KIND: &str = "morrow-owned-token";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TokenStorageSurface {
    KeychainBridge,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum KeychainErrorCode {
    UnsupportedService,
    UnsupportedTokenKind,
    StorageUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeychainBridgeError {
    code: KeychainErrorCode,
    message: String,
}

impl KeychainBridgeError {
    pub fn code(&self) -> KeychainErrorCode {
        self.code
    }

    fn unsupported_service(service: &str) -> Self {
        Self {
            code: KeychainErrorCode::UnsupportedService,
            message: format!("unsupported Keychain service: {service}"),
        }
    }

    fn unsupported_token_kind(token_kind: &str) -> Self {
        Self {
            code: KeychainErrorCode::UnsupportedTokenKind,
            message: format!("unsupported token kind: {token_kind}"),
        }
    }

    pub(crate) fn storage_unavailable(message: impl Into<String>) -> Self {
        Self {
            code: KeychainErrorCode::StorageUnavailable,
            message: message.into(),
        }
    }
}

impl fmt::Display for KeychainBridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for KeychainBridgeError {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokenLookupRequest {
    pub service: String,
    pub token_kind: String,
}

impl TokenLookupRequest {
    pub fn new(service: &str, token_kind: &str) -> Self {
        Self {
            service: service.to_owned(),
            token_kind: token_kind.to_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokenWriteRequest {
    pub service: String,
    pub token_kind: String,
    pub token: String,
}

impl TokenWriteRequest {
    pub fn new(service: &str, token_kind: &str, token: &str) -> Self {
        Self {
            service: service.to_owned(),
            token_kind: token_kind.to_owned(),
            token: token.to_owned(),
        }
    }

    fn lookup(&self) -> TokenLookupRequest {
        TokenLookupRequest {
            service: self.service.clone(),
            token_kind: self.token_kind.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokenCommandReceipt {
    pub storage_surface: TokenStorageSurface,
    pub stored: bool,
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokenReadResponse {
    pub storage_surface: TokenStorageSurface,
    pub present: bool,
    pub token: Option<String>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MorrowTokenVault;

impl MorrowTokenVault {
    pub fn create(
        &self,
        request: TokenWriteRequest,
    ) -> Result<TokenCommandReceipt, KeychainBridgeError> {
        let lookup = request.lookup();
        validate_lookup(&lookup)?;
        store_password(&lookup, &request.token)?;
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
        let token = read_password(&request)?;
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
        let deleted = delete_password(&request)?;
        Ok(TokenCommandReceipt {
            storage_surface: TokenStorageSurface::KeychainBridge,
            stored: false,
            deleted,
        })
    }
}

#[derive(Debug, Default)]
pub struct FakeMorrowTokenVault {
    token: Mutex<Option<String>>,
}

impl FakeMorrowTokenVault {
    pub fn create(
        &self,
        request: TokenWriteRequest,
    ) -> Result<TokenCommandReceipt, KeychainBridgeError> {
        validate_lookup(&request.lookup())?;
        let mut token = self.token.lock().map_err(|_| {
            KeychainBridgeError::storage_unavailable("fake Morrow token vault lock is poisoned")
        })?;
        *token = Some(request.token);
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
            .token
            .lock()
            .map_err(|_| {
                KeychainBridgeError::storage_unavailable("fake Morrow token vault lock is poisoned")
            })?
            .clone();
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
        let mut token = self.token.lock().map_err(|_| {
            KeychainBridgeError::storage_unavailable("fake Morrow token vault lock is poisoned")
        })?;
        let deleted = token.take().is_some();
        Ok(TokenCommandReceipt {
            storage_surface: TokenStorageSurface::KeychainBridge,
            stored: false,
            deleted,
        })
    }
}

fn validate_lookup(request: &TokenLookupRequest) -> Result<(), KeychainBridgeError> {
    if request.service != MORROW_KEYCHAIN_SERVICE {
        return Err(KeychainBridgeError::unsupported_service(&request.service));
    }
    if request.token_kind != MORROW_TOKEN_KIND {
        return Err(KeychainBridgeError::unsupported_token_kind(
            &request.token_kind,
        ));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn store_password(lookup: &TokenLookupRequest, token: &str) -> Result<(), KeychainBridgeError> {
    macos_keychain::store_password(lookup, token)
}

#[cfg(not(target_os = "macos"))]
fn store_password(_lookup: &TokenLookupRequest, _token: &str) -> Result<(), KeychainBridgeError> {
    Err(unsupported_platform_error())
}

#[cfg(target_os = "macos")]
fn read_password(lookup: &TokenLookupRequest) -> Result<Option<String>, KeychainBridgeError> {
    macos_keychain::read_password(lookup)
}

#[cfg(not(target_os = "macos"))]
fn read_password(_lookup: &TokenLookupRequest) -> Result<Option<String>, KeychainBridgeError> {
    Err(unsupported_platform_error())
}

#[cfg(target_os = "macos")]
fn delete_password(lookup: &TokenLookupRequest) -> Result<bool, KeychainBridgeError> {
    macos_keychain::delete_password(lookup)
}

#[cfg(not(target_os = "macos"))]
fn delete_password(_lookup: &TokenLookupRequest) -> Result<bool, KeychainBridgeError> {
    Err(unsupported_platform_error())
}

#[cfg(not(target_os = "macos"))]
fn unsupported_platform_error() -> KeychainBridgeError {
    KeychainBridgeError::storage_unavailable(
        "Morrow Keychain bridge storage requires macOS Security.framework",
    )
}
