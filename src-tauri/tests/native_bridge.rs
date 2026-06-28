use morrow_lib::native_bridge::{
    FakeNativeBridge, KeychainErrorCode, NativeBridgeState, TokenLookupRequest,
    TokenStorageSurface, TokenWriteRequest, MORROW_KEYCHAIN_SERVICE, MORROW_PROVIDER_TOKEN_KIND,
    MORROW_TOKEN_KIND,
};

#[test]
fn keychain_create_read_delete_accepts_only_morrow_owned_token() -> Result<(), String> {
    let bridge = FakeNativeBridge::default();
    let lookup = TokenLookupRequest::new(MORROW_KEYCHAIN_SERVICE, MORROW_TOKEN_KIND);
    let write = TokenWriteRequest::new(
        MORROW_KEYCHAIN_SERVICE,
        MORROW_TOKEN_KIND,
        "fixture-redacted-token-value",
    );

    let saved = bridge
        .create_morrow_token(write)
        .map_err(|error| error.to_string())?;
    let read = bridge
        .read_morrow_token(lookup.clone())
        .map_err(|error| error.to_string())?;
    let deleted = bridge
        .delete_morrow_token(lookup.clone())
        .map_err(|error| error.to_string())?;
    let after_delete = bridge
        .read_morrow_token(lookup)
        .map_err(|error| error.to_string())?;

    assert_eq!(saved.storage_surface, TokenStorageSurface::KeychainBridge);
    assert!(read.present);
    assert_eq!(read.token.as_deref(), Some("fixture-redacted-token-value"));
    assert!(deleted.deleted);
    assert!(!after_delete.present);
    assert_eq!(
        after_delete.storage_surface,
        TokenStorageSurface::KeychainBridge
    );
    Ok(())
}

#[test]
fn keychain_accepts_provider_token_kind_and_keeps_owned_token_separate() -> Result<(), String> {
    let bridge = FakeNativeBridge::default();
    let owned_lookup = TokenLookupRequest::new(MORROW_KEYCHAIN_SERVICE, MORROW_TOKEN_KIND);
    let provider_lookup =
        TokenLookupRequest::new(MORROW_KEYCHAIN_SERVICE, MORROW_PROVIDER_TOKEN_KIND);

    bridge
        .create_morrow_token(TokenWriteRequest::new(
            MORROW_KEYCHAIN_SERVICE,
            MORROW_TOKEN_KIND,
            "fixture-owned-redacted-token",
        ))
        .map_err(|error| error.to_string())?;
    bridge
        .create_morrow_token(TokenWriteRequest::new(
            MORROW_KEYCHAIN_SERVICE,
            MORROW_PROVIDER_TOKEN_KIND,
            "fixture-provider-redacted-token",
        ))
        .map_err(|error| error.to_string())?;

    let owned = bridge
        .read_morrow_token(owned_lookup.clone())
        .map_err(|error| error.to_string())?;
    let provider = bridge
        .read_morrow_token(provider_lookup.clone())
        .map_err(|error| error.to_string())?;
    let provider_deleted = bridge
        .delete_morrow_token(provider_lookup.clone())
        .map_err(|error| error.to_string())?;
    let owned_after_provider_delete = bridge
        .read_morrow_token(owned_lookup)
        .map_err(|error| error.to_string())?;
    let provider_after_delete = bridge
        .read_morrow_token(provider_lookup)
        .map_err(|error| error.to_string())?;

    assert_eq!(owned.token.as_deref(), Some("fixture-owned-redacted-token"));
    assert_eq!(
        provider.token.as_deref(),
        Some("fixture-provider-redacted-token")
    );
    assert!(provider_deleted.deleted);
    assert_eq!(
        owned_after_provider_delete.token.as_deref(),
        Some("fixture-owned-redacted-token")
    );
    assert!(!provider_after_delete.present);
    Ok(())
}

#[test]
fn provider_readiness_ignores_legacy_owned_token() -> Result<(), String> {
    let bridge = FakeNativeBridge::default();

    bridge
        .create_morrow_token(TokenWriteRequest::new(
            MORROW_KEYCHAIN_SERVICE,
            MORROW_TOKEN_KIND,
            "fixture-owned-redacted-token",
        ))
        .map_err(|error| error.to_string())?;

    let provider = bridge
        .read_morrow_token(TokenLookupRequest::new(
            MORROW_KEYCHAIN_SERVICE,
            MORROW_PROVIDER_TOKEN_KIND,
        ))
        .map_err(|error| error.to_string())?;

    assert!(!provider.present);
    assert!(provider.token.is_none());
    Ok(())
}

#[test]
fn keychain_rejects_non_morrow_service_and_unsupported_token_kind() {
    let bridge = FakeNativeBridge::default();
    let non_morrow_service = TokenWriteRequest::new(
        "com.example.not-morrow",
        MORROW_TOKEN_KIND,
        "fixture-redacted-token-value",
    );
    let unsupported_kind = TokenWriteRequest::new(
        MORROW_KEYCHAIN_SERVICE,
        "unsupported-token-kind",
        "fixture-redacted-token-value",
    );

    let service_error = bridge.create_morrow_token(non_morrow_service);
    let kind_error = bridge.create_morrow_token(unsupported_kind);

    assert_eq!(
        service_error.map_err(|error| error.code()),
        Err(KeychainErrorCode::UnsupportedService)
    );
    assert_eq!(
        kind_error.map_err(|error| error.code()),
        Err(KeychainErrorCode::UnsupportedTokenKind)
    );
}

#[test]
fn keychain_bridge_sources_do_not_contain_file_or_browser_storage_fallbacks() {
    let sources = [
        include_str!("../src/native_bridge/keychain.rs"),
        include_str!("../src/native_bridge/keychain/macos_keychain.rs"),
        include_str!("../src/native_bridge/fake.rs"),
    ];
    let forbidden_fallbacks = [
        "std::fs",
        "fs::",
        "File::",
        "OpenOptions",
        "localStorage",
        "sessionStorage",
        "IndexedDB",
    ];

    for source in sources {
        for forbidden in forbidden_fallbacks {
            assert!(
                !source.contains(forbidden),
                "native token bridge source contains forbidden fallback marker {forbidden}"
            );
        }
    }
}

#[test]
fn keychain_delete_command_layer_smoke() -> Result<(), String> {
    let state = NativeBridgeState::with_bridge(FakeNativeBridge::default());
    let lookup = TokenLookupRequest::new(MORROW_KEYCHAIN_SERVICE, MORROW_TOKEN_KIND);

    state
        .create_morrow_token(TokenWriteRequest::new(
            MORROW_KEYCHAIN_SERVICE,
            MORROW_TOKEN_KIND,
            "fixture-redacted-token-value",
        ))
        .map_err(|error| error.to_string())?;
    let before_delete = state
        .read_morrow_token(lookup.clone())
        .map_err(|error| error.to_string())?;
    let deleted = state
        .delete_morrow_token(lookup.clone())
        .map_err(|error| error.to_string())?;
    let after_delete = state
        .read_morrow_token(lookup)
        .map_err(|error| error.to_string())?;

    assert_eq!(
        before_delete.storage_surface,
        TokenStorageSurface::KeychainBridge
    );
    assert!(before_delete.present);
    assert!(deleted.deleted);
    assert_eq!(
        after_delete.storage_surface,
        TokenStorageSurface::KeychainBridge
    );
    assert!(!after_delete.present);
    Ok(())
}

#[test]
fn production_keychain_rejects_malformed_lookup_before_storage() {
    let state = NativeBridgeState::default();
    let non_morrow_service = TokenWriteRequest::new(
        "com.example.not-morrow",
        MORROW_TOKEN_KIND,
        "fixture-redacted-token-value",
    );
    let unsupported_kind = TokenWriteRequest::new(
        MORROW_KEYCHAIN_SERVICE,
        "unsupported-token-kind",
        "fixture-redacted-token-value",
    );

    let service_error = state.create_morrow_token(non_morrow_service);
    let kind_error = state.create_morrow_token(unsupported_kind);

    assert_eq!(
        service_error.map_err(|error| error.code()),
        Err(KeychainErrorCode::UnsupportedService)
    );
    assert_eq!(
        kind_error.map_err(|error| error.code()),
        Err(KeychainErrorCode::UnsupportedTokenKind)
    );
}
